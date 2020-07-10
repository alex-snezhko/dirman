use std::env;
use std::fs::{self, Metadata};
use std::os::windows::prelude::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::ops::{Add, AddAssign, Sub};
use std::cmp::max;
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
    //size: String,
    //last_modified: String,
    // TODO metadata
}

struct Directory {
    name: OsString,
    meta: Metadata,
    files: Vec<FileInfo>,
    directories: Vec<Directory>,
}

struct ScrollableArea {
    screen_offset: Vector2,  // the location on the terminal window of the top left of this area
    size: Vector2,           // the size of the area where content may be drawn (not including border)
    curr_pos: Vector2,       // current offset into the content
    contents: Vec<String>,   // buffer of all contents that can be printed in this area
}

impl ScrollableArea {
    fn draw(&self, term: &Term) -> io::Result<()> {
        #[derive(Clone, Copy)]
        enum Direction {
            Up,
            Right,
            Down,
            Left,
        }

        let draw_arrows = |direction| -> io::Result<()> {
            let (arrow, plus_x, plus_y, begin_offset, count): (_, isize, isize, _, _) = match direction {
                Direction::Up => ("↑", 4, 0, Vector2 { x: 0, y: 0 }, self.size.x / 4),
                Direction::Right => ("→", 0, 2, Vector2 { x: self.size.x, y: 0 }, self.size.y / 2),
                Direction::Down => ("↓", -4, 0, Vector2 { x: self.size.x, y: self.size.y + 1 }, self.size.x / 4),
                Direction::Left => ("←", 0, -2, Vector2 { x: 0, y: self.size.y + 1 }, self.size.y / 2),
            };

            // subtract (1,1) from the screen offset because it is inside the border
            let mut pos = self.screen_offset - Vector2 { x: 1, y: 1 } + begin_offset;
            for _ in 0..count {
                term.move_cursor_to(pos.x, pos.y)?;
                term.write_str(arrow)?;
                term.move_cursor_left(1)?;

                pos.x = pos.x.wrapping_add(plus_x as usize);
                pos.y = pos.y.wrapping_add(plus_y as usize);
            }
            Ok(())
        };
        
        if self.curr_pos.y > 0 {
            draw_arrows(Direction::Up)?;
        }

        if self.size.x - self.curr_pos.x <= self.contents.iter()
            .fold(0, |largest, x| max(largest, x.chars().count()))
        {
            draw_arrows(Direction::Right)?;
        }

        if self.size.y - self.curr_pos.y <= self.contents.len() {
            draw_arrows(Direction::Down)?;
        }

        if self.curr_pos.x > 0 {
            draw_arrows(Direction::Left)?;
        }

        for i in 0..self.size.y {
            let to_print: String = match self.contents.get(self.curr_pos.y + i) {
                Some(s) => s.chars().skip(self.curr_pos.x).take(self.size.x).collect(),
                None => break,
            };

            term.move_cursor_to(self.screen_offset.x, self.screen_offset.y + i)?;
            term.write_str(&to_print)?;
        }

        Ok(())
    }
}

// #[derive(Clone, Copy)]
// enum Which { Command, Tree, Contents }

// struct CurrentArea {
//     which: Which,
//     area: ScrollableArea,
// }

#[derive(Clone, Copy, PartialEq)]
enum CurrentArea { Command, Tree, Contents }

struct TerminalManager<'a> {
    term: &'a Term,
    root: &'a Directory,
    curr_dir: &'a Directory,
    tree_area: ScrollableArea,
    contents_area: ScrollableArea,
    command_area: ScrollableArea,
}

impl<'a> TerminalManager<'a> {
    fn process_command(&self, command: &str) {
        let tokens: Vec<&str> = command.split_whitespace().collect();
        if tokens.len() == 0 {
            return;
        }

        match tokens[0] {
            "enter" => {
                if tokens.len() > 2 {
                    //print_error("'enter' command must be followed by a directory");
                }
                let possible_dirs = self.to_directory(tokens[1]);
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

    fn print_error(&self, message: &str) {
        //self.term.move_cursor_to(x: usize, y: usize)
    }

    fn highlight_if_multiple(&self, dirs: &Vec<&'a Directory>) {
        if dirs.len() > 1 {
            //self.draw_tree(dirs, &mut 1, self.root, &mut Vector2 { x: 0, y: 0 });
        }
    }

    fn to_directory(&'a self, path: &str) -> Vec<&'a Directory> {
        let parts: Vec<&str> = path.split("/").collect();
        
        let mut possible = vec![self.root];
        for part in &parts {
            let mut new: Vec<&'a Directory> = vec![];
            for dir in &possible {
                Self::to_directory_helper(part, dir, &mut new);
            }
            possible = new;
        }
        
        possible
    }

    fn to_directory_helper(path: &str, curr_dir: &'a Directory, possible: &mut Vec<&'a Directory>) {
        for dir in &curr_dir.directories {
            if dir.name == path {
                possible.push(dir);
            }
            Self::to_directory_helper(path, dir, possible);
        }
    }

    fn move_file(&self, file: &FileInfo, old_dir: &Directory, new_dir: &Directory) {
        //fs::
    }

    fn draw_outline(&self, panel: CurrentArea) -> io::Result<()> {
        let (height, width) = {
            let size = self.term.size();
            (size.0 as usize, size.1 as usize)
        };

        let line_x = (width as f64 * 0.6) as usize;
    
        self.term.move_cursor_to(0, 0)?;
        self.term.write_line("DirMan")?;

        let color = Style::new().red();
        let print_with_color = |text: &str, when: Vec<CurrentArea>| -> io::Result<()> {
            let colored = if when.contains(&panel) {
                format!("{}", color.apply_to(text))
            } else {
                text.to_string()
            };
            self.term.write_str(&colored)?;
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
        self.term.write_line("")?;
        
        for _ in 0..height-4 {
            for _ in 0..line_x {
                self.term.move_cursor_right(line_x)?;
            }
            print_with_color("┃", vec![Tree, Contents])?;
            self.term.write_line("")?;
        }
    
        for _ in 0..line_x {
            print_with_color("━", vec![Tree, Command])?;
        }
        print_with_color("┻", vec![Tree, Contents, Command])?;
        for _ in line_x+1..width {
            print_with_color("━", vec![Contents, Command])?;
        }
        self.term.write_line("")?;

        print!(" > ");
        io::stdout().flush()?;

        Ok(())
    }

    fn load_tree(&mut self) -> io::Result<()> {
        let mut content = vec![String::new()];
        self.load_tree_rec(&mut content, &vec![], &mut 0, self.root, vec![])?;
        self.tree_area.contents = content;
        Ok(())
    }

    // TODO maybe make this a little cleaner by returning a subtree; will no longer need 'content' or 'pipe_needed_at' parameters
    fn load_tree_rec(
        &self,
        content: &mut Vec<String>,  // buffer of all contents in tree
        selected_dirs: &Vec<&'a Directory>,
        selected_dir_num: &mut i32, // counter for when multiple directories are selected, to display each with own number
        curr_dir: &'a Directory,    // current dir being processed
        pipe_needed_at: Vec<bool>,  // vector determining whether a pipe (for branch) is needed at an offset
    ) -> io::Result<()>
    {
        let curr_dir_name: &str = curr_dir.name.to_str().unwrap();

        let curr_line: &mut String = content.last_mut().unwrap();

        // print item with correct color
        if selected_dirs.iter().any(|&e| e as *const Directory == curr_dir as *const Directory) {
            // print a selected item in red
            let text = format!("{}: {}", curr_dir_name, selected_dir_num);
            *selected_dir_num += 1;
            curr_line.push_str(&format!("{}", Style::new().red().apply_to(text)));
        } else if curr_dir as *const Directory == self.curr_dir as *const Directory {
            // print current directory in blue
            curr_line.push_str(&format!("{}", Style::new().blue().apply_to(curr_dir_name)));
        } else {
            // otherwise print normally
            curr_line.push_str(curr_dir_name);
        }

        // iterate through all child directories and load them as well
        let dirs = &curr_dir.directories;
        for (i, dir) in dirs.iter().enumerate() {
            content.push(String::new());
            let new_line = content.last_mut().unwrap();

            // insert pipes when needed into contents
            for e in &pipe_needed_at {
                new_line.push_str(if *e { "│  " } else { "   " });
            }

            // next index will need pipe if it is not the last one in branch
            let mut next_pipe_needed_at = pipe_needed_at.clone();
            if i == dirs.len() - 1 {
                new_line.push_str("└─ ");
                next_pipe_needed_at.push(false);
            } else {
                new_line.push_str("├─ ");
                next_pipe_needed_at.push(true);
            }

            self.load_tree_rec(content, selected_dirs, selected_dir_num, dir, next_pipe_needed_at)?;
        }

        Ok(())
    }

    fn load_contents(&mut self) -> io::Result<()> {
        let contents = &mut self.contents_area.contents;
        contents.clear();

        contents.push("Last Modified           Size  Name".to_string());
        contents.push("-------------           ----  ----".to_string());
        if self.curr_dir.directories.len() != 0 {
            contents.push("- Directories -".to_string());

            for dir in &self.curr_dir.directories {
                let last_mod = DateTime::<Utc>::from(dir.meta.modified()?);

                let (pm, hour) = last_mod.hour12();
                contents.push(format!("{:02}/{:02}/{:02} {:02}:{:02} {}           {}",
                    last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                    hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                    dir.name.to_str().unwrap()));                            // file name
            }
            contents.push(String::new());
        }

        if self.curr_dir.files.len() != 0 {
            contents.push("- Files -".to_string());

            for file in &self.curr_dir.files {
                let last_mod = DateTime::<Utc>::from(file.meta.modified()?);

                let (pm, hour) = last_mod.hour12();
                contents.push(format!("{:02}/{:02}/{:02} {:02}:{:02} {}  {:>7}  {}",
                    last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                    hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                    file_size_to_str(file.meta.file_size()),                 // file size string
                    file.name.to_str().unwrap()));                           // file name
            }
        }
        Ok(())
    }
}

fn file_size_to_str(size: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if size >= GB {
        format!("{:.0} GB", size as f64 / GB as f64) // TODO change to usize
    } else if size >= MB {
        format!("{:.0} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.0} KB", size as f64 / KB as f64)
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

    let line_x = (size.x as f64 * 0.6) as usize;
    
    let mut manager = TerminalManager {
        term: &term,
        root: &root,
        curr_dir: &root,

        tree_area: ScrollableArea {
            screen_offset: Vector2 { x: 1, y: 3 },
            size: Vector2 { x: line_x - 3, y: size.y - 6 },
            curr_pos: Vector2 { x: 0, y: 0 },
            contents: vec![],
        },

        contents_area: ScrollableArea {
            screen_offset: Vector2 { x: line_x + 2, y: 3 },
            size: Vector2 { x: (size.x - line_x) - 3, y: size.y - 6 },
            curr_pos: Vector2 { x: 0, y: 0 },
            contents: vec![],
        },

        command_area: ScrollableArea {
            screen_offset: Vector2 { x: 3, y: size.y - 2 },
            size: Vector2 { x: size.x - 3, y: 1 },
            curr_pos: Vector2 { x: 0, y: 0 },
            contents: vec![],
        },
    };

    for _ in 0..size.y {
        term.write_line("")?;
    }

    manager.draw_outline(CurrentArea::Command)?;

    manager.load_tree()?;
    manager.load_contents()?;
    
    manager.tree_area.draw(manager.term)?;
    manager.contents_area.draw(manager.term)?;

    term.move_cursor_to(3, size.y - 1)?;

    let mut curr_area = (CurrentArea::Command, &mut manager.command_area);
    
    let mut command = String::new();
    loop {
        let key = term.read_key()?;

        use console::Key::*;
        match key {
            ArrowUp => {
                if let CurrentArea::Command = curr_area.0 {
                    curr_area = (CurrentArea::Tree, &mut manager.tree_area);
                    manager.draw_outline(CurrentArea::Tree)?;
                    manager.term.hide_cursor()?;
                }
            },
            ArrowRight => {
                if let CurrentArea::Tree = curr_area.0 {
                    curr_area = (CurrentArea::Contents, &mut manager.contents_area);
                    manager.draw_outline(CurrentArea::Contents)?;
                }
            },
            ArrowDown | Escape => {
                if curr_area.0 != CurrentArea::Command {
                    curr_area = (CurrentArea::Command, &mut manager.command_area);
                    manager.draw_outline(CurrentArea::Command)?;
                    manager.term.show_cursor()?;
                }
            },
            ArrowLeft => {
                if let CurrentArea::Contents = curr_area.0 {
                    curr_area = (CurrentArea::Tree, &mut manager.tree_area);
                    manager.draw_outline(CurrentArea::Tree)?;
                }
            },
            Char(c) => {
                match curr_area.0 {
                    CurrentArea::Command => {
                        command.push(c);
                        term.write_str(&c.to_string())?;
                    }
                    _ => {
                        match c {
                            'w' | 'W' => {
                                let area = &mut curr_area.1;
                                area.curr_pos.y += 5;
                            },
                            'a' | 'A' => {},
                            's' | 'S' => {},
                            'd' | 'D' => {},
                            _ => {},
                        }
                    }
                }
            },
            Enter => {
                if command == "q" {
                    break;
                }
                manager.process_command(&command);
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

fn print_to_area(term: &Term, bounds: &ScrollableArea, text: &str, color: Style, pos: Vector2) -> io::Result<()> {
    #[derive(Clone, Copy)]
    enum Direction {
        Up,
        Right,
        Down,
        Left,
    }

    let draw_arrows = |direction| -> io::Result<()> {
        let (arrow, plus_x, plus_y, begin_offset, count): (_, isize, isize, _, _) = match direction {
            Direction::Up => ("↑", 4, 0, Vector2 { x: 0, y: 0 }, bounds.size.x / 4),
            Direction::Right => ("→", 0, 2, Vector2 { x: bounds.size.x - 1, y: 0 }, bounds.size.y / 2),
            Direction::Down => ("↓", -4, 0, Vector2 { x: bounds.size.x - 1, y: bounds.size.y }, bounds.size.x / 4),
            Direction::Left => ("←", 0, -2, Vector2 { x: 0, y: bounds.size.y }, bounds.size.y / 2),
        };

        let mut pos = bounds.screen_offset + begin_offset;
        for _ in 0..count {
            term.move_cursor_to(pos.x, pos.y)?;
            term.write_str(arrow)?;
            term.move_cursor_left(1)?;

            pos.x = pos.x.wrapping_add(plus_x as usize);
            pos.y = pos.y.wrapping_add(plus_y as usize);
        }
        Ok(())
    };

    let actual_pos = pos + bounds.screen_offset - bounds.curr_pos;

    let low_bound = bounds.screen_offset;
    let high_bound = bounds.screen_offset + bounds.size;

    let in_bounds_top = actual_pos.y >= low_bound.y;
    let in_bounds_right = actual_pos.x + text.len() < high_bound.x;
    let in_bounds_bot = actual_pos.y < high_bound.y;
    let in_bounds_left = actual_pos.x >= low_bound.x;
    
    if !in_bounds_top {
        draw_arrows(Direction::Up)?;
    }

    if !in_bounds_right {
        draw_arrows(Direction::Right)?;
    }

    if !in_bounds_bot {
        draw_arrows(Direction::Down)?;
    }

    if !in_bounds_left {
        draw_arrows(Direction::Left)?;
    }

    if in_bounds_top && in_bounds_bot {
        // text may have been cut off; allow for adjusting of what's printed if text is cut off
        let mut text: &str = text;
        let mut begin_x = actual_pos.x;
        if !in_bounds_left {
            text = &text[actual_pos.x-low_bound.x..];
            begin_x = low_bound.x;
        }
        if !in_bounds_right {
            text = &text[..high_bound.x-actual_pos.x];
        }
        term.move_cursor_to(begin_x, actual_pos.y)?;
        term.write_str(&format!("{}", color.apply_to(text)))?;
    }

    Ok(())
}
