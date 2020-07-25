use std::env;
use std::fs::{self, Metadata};
use std::os::windows::prelude::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::ops::{Add, AddAssign, Sub};
use std::cmp::{max, min};
use console::{Term, Style};
use chrono::{DateTime, Utc, Datelike, Timelike};

#[derive(Debug, Clone, Copy)]
struct Vector2 {
    x: usize,
    y: usize,
}

impl Add for Vector2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl AddAssign for Vector2 {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

impl Sub for Vector2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

struct FileInfo {
    name: OsString,
    meta: Metadata,
}

struct Directory {
    name: OsString,
    meta: Metadata,
    files: Vec<FileInfo>,
    directories: Vec<Directory>,
}

struct ColoredString {
    string: String,
    color: Style,
}

struct ScrollableArea {
    screen_offset: Vector2,  // the location on the terminal window of the top left of this area
    size: Vector2,           // the size of the area
    curr_pos: Vector2,       // current offset into the content
    contents: Vec<String>,   // buffer of all contents that can be printed in this area
    longest_line_len: usize, // cache: length of longest line in contents
}

impl ScrollableArea {
    fn contents_size(&self) -> Vector2 {
        self.size - Vector2 { x: 4, y: 2 }
    }

    fn draw(&self, term: &Term) -> io::Result<()> {
        #[derive(Clone, Copy)]
        enum ArrowLocation {
            Top,
            Right,
            Bottom,
            Left,
        }
        let contents_size = self.contents_size();

        // clear the panel
        // let end_x = self.screen_offset.x + self.size.x + 4;
        // for y in 0..self.size.y+2 {
        //     term.move_cursor_to(end_x, self.screen_offset.y + y)?;
        //     term.clear_chars(self.size.x + 4)?;
        // }
        for y in 0..self.size.y {
            term.move_cursor_to(self.screen_offset.x, self.screen_offset.y + y)?;
            for _ in 0..self.size.x {
                term.write_str(" ")?;
            }
        }

        // closure for drawing arrows in specified direction
        let draw_arrows = |direction| -> io::Result<()> {
            // bounds that arrows may be drawn in will be 1 unit away from edges
            let s = self.size - Vector2 { x: 2, y: 2 };

            use ArrowLocation::*;
            // begin_offset determines where to place the terminal cursor for the first arrow
            // in the sequence; it is calculated by the principle that as many arrows should be
            // drawn as can fit in the bounds; the complex calculation involves finding the size of
            // the "line" of arrows to be drawn (including one extra as fence-post) and centering it
            let (arrow, horizontal, begin_offset, count) = match direction {
                Top => ("↑", true, Vector2 { x: 1 + (s.x - (((s.x - 1) / 4) * 4 + 1)) / 2, y: 0 }, (s.x - 1) / 4 + 1),
                Right => ("→", false, Vector2 { x: s.x + 1, y: 1 + (s.y - (((s.y - 1) / 2) * 2 + 1)) / 2 }, (s.y - 1) / 2 + 1),
                Bottom =>  ("↓", true, Vector2 { x: 1 + (s.x - (((s.x - 1) / 4) * 4 + 1)) / 2, y: s.y + 1 }, (s.x - 1) / 4 + 1),
                Left => ("←", false, Vector2 { x: 0, y: 1 + (s.y - (((s.y - 1) / 2) * 2 + 1)) / 2 }, (s.y - 1) / 2 + 1),
            };

            let mut pos = self.screen_offset + begin_offset;
            for _ in 0..count {
                term.move_cursor_to(pos.x, pos.y)?;
                term.write_str(arrow)?;

                if horizontal {
                    pos.x += 4;
                } else {
                    pos.y += 2;
                }
            }
            Ok(())


            // use Direction::*;
            // let (arrow, plus_x, plus_y, begin_offset, count): (_, isize, isize, _, _) = match direction {
            //     Up => ("↑", 4, 0, Vector2 { x: 0, y: 0 }, self.size.x / 4),
            //     Right => ("→", 0, 2, Vector2 { x: self.size.x + 3, y: 0 }, self.size.y / 2),
            //     Down => ("↓", -4, 0, Vector2 { x: self.size.x + 3, y: self.size.y + 1 }, self.size.x / 4),
            //     Left => ("←", 0, -2, Vector2 { x: 0, y: self.size.y + 1 }, self.size.y / 2),
            // };

            // let mut pos = self.screen_offset + begin_offset;
            // for _ in 0..count {
            //     term.move_cursor_to(pos.x, pos.y)?;
            //     term.write_str(arrow)?;
            //     term.move_cursor_left(1)?;

            //     pos.x = pos.x.wrapping_add(plus_x as usize);
            //     pos.y = pos.y.wrapping_add(plus_y as usize);
            // }
            // Ok(())
        };
        
        if self.curr_pos.y > 0 {
            draw_arrows(ArrowLocation::Top)?;
        }

        if contents_size.x + self.curr_pos.x < self.longest_line_len {
            draw_arrows(ArrowLocation::Right)?;
        }

        if contents_size.y + self.curr_pos.y < self.contents.len() {
            draw_arrows(ArrowLocation::Bottom)?;
        }

        if self.curr_pos.x > 0 {
            draw_arrows(ArrowLocation::Left)?;
        }

        for i in 0..contents_size.y {
            let to_print: String = match self.contents.get(self.curr_pos.y + i) {
                Some(s) => s.chars().skip(self.curr_pos.x).take(contents_size.x).collect(),
                None => break,
            };

            term.move_cursor_to(self.screen_offset.x + 2, self.screen_offset.y + 1 + i)?;
            term.write_str(&to_print)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CurrentArea { Command, Tree, Contents }

fn process_command(root: &Directory, command: &str) {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    if tokens.len() == 0 {
        return;
    }

    match tokens[0] {
        "enter" => {
            if tokens.len() > 2 {
                //print_error("'enter' command must be followed by a directory");
            }
            let possible_dirs = to_directory(root, tokens[1]);
            //Self::highlight_if_multiple(&possible_dirs);
        },
        "close" => {},
        "open" => {},
        "move" => {
            if tokens.len() > 2 {
                //print_error("'enter' command must be followed by a directory");
            }
        },
        "rename" => {},
        "copy" => {},
        "new" => {},
        _ => {},
    }
}

// fn highlight_if_multiple(&self, dirs: &Vec<&'a Directory>) {
//     if dirs.len() > 1 {
//         //self.draw_tree(dirs, &mut 1, self.root, &mut Vector2 { x: 0, y: 0 });
//     }
// }

fn to_directory<'a>(root: &'a Directory, path: &str) -> Vec<&'a Directory> {
    let parts: Vec<&str> = path.split("/").collect();
    
    let mut possible = vec![root];
    for part in &parts {
        let mut new: Vec<&'a Directory> = vec![];
        for dir in &possible {
            to_directory_helper(part, dir, &mut new);
        }
        possible = new;
    }
    
    possible
}

fn to_directory_helper<'a>(path: &str, curr_dir: &'a Directory, possible: &mut Vec<&'a Directory>) {
    for dir in &curr_dir.directories {
        if dir.name == path {
            possible.push(dir);
        }
        to_directory_helper(path, dir, possible);
    }
}

// fn move_file(&self, file: &FileInfo, old_dir: &Directory, new_dir: &Directory) {
//     //fs::
// }

fn draw_outline(term: &Term, panel: CurrentArea) -> io::Result<()> {
    let (height, width) = {
        let size = term.size();
        (size.0 as usize, size.1 as usize)
    };

    let line_x = (width as f64 * 0.6) as usize;

    term.move_cursor_to(0, 0)?;
    term.write_line("DirMan")?;

    let red = Style::new().red();
    let print_with_color = |text: &str, colored_list: Vec<CurrentArea>| -> io::Result<()> {
        let colored = if colored_list.contains(&panel) {
            format!("{}", red.apply_to(text))
        } else {
            text.to_string()
        };
        term.write_str(&colored)?;
        Ok(())
    };
    
    use CurrentArea::*;

    for _ in 0..line_x {
        print_with_color("━", vec![Tree])?;
    }
    print_with_color("┳", vec![Tree, Contents])?;
    for _ in line_x+1..width {
        print_with_color("━", vec![Contents])?;
    }
    term.write_line("")?;
    
    for _ in 0..height-4 {
        for _ in 0..line_x {
            term.move_cursor_right(line_x)?;
        }
        print_with_color("┃", vec![Tree, Contents])?;
        term.write_line("")?;
    }

    for _ in 0..line_x {
        print_with_color("━", vec![Tree, Command])?;
    }
    print_with_color("┻", vec![Tree, Contents, Command])?;
    for _ in line_x+1..width {
        print_with_color("━", vec![Contents, Command])?;
    }
    term.write_line("")?;

    print!(" > ");
    io::stdout().flush()?;

    Ok(())
}

fn load_tree(
    selected_dir: &Directory,
    selected_dirs: &Vec<&Directory>,
    selected_dir_num: &mut i32, // counter for when multiple directories are selected, to display each with own number
    curr_dir: &Directory,    // current dir being processed
) -> Vec<String>
{
    let mut contents = vec![];

    let curr_dir_name: &str = curr_dir.name.to_str().unwrap();

    // print item with correct color
    contents.push(
        if selected_dirs.iter().any(|&e| e as *const Directory == curr_dir as *const Directory) {
            // print a selected item in red
            let text = format!("{}: {}", curr_dir_name, selected_dir_num);
            *selected_dir_num += 1;
            format!("{}", Style::new().red().apply_to(text))
        } else if curr_dir as *const Directory == selected_dir as *const Directory {
            // print current directory in blue
            format!("{}", Style::new().blue().apply_to(curr_dir_name))
        } else {
            // otherwise print normally
            curr_dir_name.to_string()
        });

    // iterate through all child directories and load them as well
    let dirs = &curr_dir.directories;
    for (i, dir) in dirs.iter().enumerate() {
        // next index will need pipe if it is not the last one in branch
        let (begin, next) = if i == dirs.len() - 1 {
            ("└─ ", "   ")
        } else {
            ("├─ ", "│  ")
        };

        let inner_dir_content = load_tree(selected_dir, selected_dirs, selected_dir_num, dir);
        for i in 0..inner_dir_content.len() {
            contents.push(String::from(if i == 0 { begin } else { next }) + &inner_dir_content[i]);
        }
    }

    contents
}

fn load_contents(curr_dir: &Directory) -> Vec<String> {
    let mut contents = vec![];

    contents.push("Last Modified           Size  Name".to_string());
    contents.push("-------------           ----  ----".to_string());
    if curr_dir.directories.len() != 0 {
        contents.push("- Directories -".to_string());

        for dir in &curr_dir.directories {
            let last_mod = DateTime::<Utc>::from(dir.meta.modified().unwrap());

            let (pm, hour) = last_mod.hour12();
            contents.push(format!("{:02}/{:02}/{:02} {:02}:{:02} {}           {}",
                last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                dir.name.to_str().unwrap()));                            // file name
        }
        contents.push(String::new());
    }

    if curr_dir.files.len() != 0 {
        contents.push("- Files -".to_string());

        for file in &curr_dir.files {
            let last_mod = DateTime::<Utc>::from(file.meta.modified().unwrap());

            let (pm, hour) = last_mod.hour12();
            contents.push(format!("{:02}/{:02}/{:02} {:02}:{:02} {}  {:>7}  {}",
                last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                file_size_to_str(file.meta.file_size()),                 // file size string
                file.name.to_str().unwrap()));                           // file name
        }
    }

    contents
}

fn file_size_to_str(size: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if size >= GB {
        format!("{} GB", size / GB)
    } else if size >= MB {
        format!("{} MB", size / MB)
    } else if size >= KB {
        format!("{} KB", size / KB)
    } else {
        format!("{} B", size)
    }
}

fn main() -> io::Result<()> {
    // parse command line arguments and extract directory
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("usage: dirman <root directory>");
        // TODO exit
    }

    let dir_str: &String = &args[1];
    let path: &Path = Path::new(dir_str);

    if !path.is_dir() {
        println!("input directory does not exist");
        return Ok(());
    }

    // construct directory tree
    let root = load_dir(path)?;

    let term = Term::stdout();

    // find dimensions for screen areas
    let size = Vector2 { x: term.size().1 as usize, y: term.size().0 as usize };

    let curr_dir = &root;

    let line_x = (size.x as f64 * 0.6) as usize;

    let tree_contents = load_tree(&root, &vec![], &mut 0, &root);
    let mut tree_area = ScrollableArea {
        screen_offset: Vector2 { x: 0, y: 2 },
        size: Vector2 { x: line_x, y: size.y - 4 },
        curr_pos: Vector2 { x: 0, y: 0 },
        longest_line_len: tree_contents.iter().fold(0, |largest, x| max(largest, x.chars().count())),
        contents: tree_contents,
    };

    let dir_contents = load_contents(&root);
    let mut contents_area = ScrollableArea {
        screen_offset: Vector2 { x: line_x + 1, y: 2 },
        size: Vector2 { x: (size.x - line_x - 1), y: size.y - 4 },
        curr_pos: Vector2 { x: 0, y: 0 },
        longest_line_len: dir_contents.iter().fold(0, |largest, x| max(largest, x.chars().count())),
        contents: dir_contents,
    };

    let mut command_area = ScrollableArea {
        screen_offset: Vector2 { x: 3, y: size.y - 2 },
        size: Vector2 { x: size.x - 3, y: 1 },
        curr_pos: Vector2 { x: 0, y: 0 },
        contents: vec![],
        longest_line_len: 0,
    };

    for _ in 0..size.y {
        term.write_line("")?;
    }

    draw_outline(&term, CurrentArea::Command)?;

    tree_area.draw(&term)?;
    contents_area.draw(&term)?;

    term.move_cursor_to(3, size.y - 1)?;

    let mut curr_area = &mut command_area;
    let mut curr_area_tag = CurrentArea::Command;
    
    let mut command = String::new();
    loop {
        let key = term.read_key()?;

        use console::Key::*;
        match key {
            ArrowUp => {
                if let CurrentArea::Command = curr_area_tag {
                    curr_area = &mut tree_area;
                    curr_area_tag = CurrentArea::Tree;
                    draw_outline(&term, CurrentArea::Tree)?;
                    term.hide_cursor()?;
                }
            },
            ArrowRight => {
                if let CurrentArea::Tree = curr_area_tag {
                    curr_area = &mut contents_area;
                    curr_area_tag = CurrentArea::Contents;
                    draw_outline(&term, CurrentArea::Contents)?;
                }
            },
            ArrowDown | Escape => {
                if curr_area_tag != CurrentArea::Command {
                    curr_area = &mut command_area;
                    curr_area_tag = CurrentArea::Command;
                    draw_outline(&term, CurrentArea::Command)?;
                    term.show_cursor()?;
                }
            },
            ArrowLeft => {
                if let CurrentArea::Contents = curr_area_tag {
                    curr_area = &mut tree_area;
                    curr_area_tag = CurrentArea::Tree;
                    draw_outline(&term, CurrentArea::Tree)?;
                }
            },
            Char(c) => {
                if let CurrentArea::Command = curr_area_tag {
                    command.push(c);
                    term.write_str(&c.to_string())?;
                } else {
                    match c {
                        'w' | 'W' => if curr_area.curr_pos.y != 0 {
                            curr_area.curr_pos.y -= min(curr_area.curr_pos.y, 5);
                            curr_area.draw(&term)?;
                        },
                        'a' | 'A' => if curr_area.curr_pos.x != 0 {
                            curr_area.curr_pos.x -= min(curr_area.curr_pos.x, 5);
                            curr_area.draw(&term)?;
                        },
                        's' | 'S' => if curr_area.contents_size().y + curr_area.curr_pos.y < curr_area.contents.len() {
                            curr_area.curr_pos.y += min(
                                curr_area.contents.len() - curr_area.contents_size().y - curr_area.curr_pos.y,
                                5);
                            curr_area.draw(&term)?;
                        },
                        'd' | 'D' => if curr_area.contents_size().x + curr_area.curr_pos.x < curr_area.longest_line_len {
                            curr_area.curr_pos.x += min(
                                curr_area.longest_line_len - curr_area.contents_size().x - curr_area.curr_pos.x,
                                5);
                            curr_area.draw(&term)?;
                        },
                        _ => {},
                    }
                }
            },
            Enter => {
                if command == "q" {
                    break;
                }
                process_command(&root, &command);
            }
            Backspace => {
                command.pop();
                term.clear_chars(1)?;
            }
            _ => {},
        }
    }
    term.clear_screen()?;

    Ok(())
}

fn load_dir(dir_path: &Path) -> io::Result<Directory> {
    let mut directories: Vec<Directory> = vec![];
    let mut files: Vec<FileInfo> = vec![];

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        
        if entry.file_type()?.is_dir() {
            directories.push(load_dir(&entry.path())?);
        } else {
            files.push(FileInfo { name: entry.file_name(), meta: entry.metadata()? });
        }
    }

    Ok(Directory {
        name: dir_path.file_name().unwrap().to_os_string(),
        meta: dir_path.metadata()?,
        directories,
        files
    })
}
