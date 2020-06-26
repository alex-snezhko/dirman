use std::env;
use std::fs;
use std::os::windows::prelude::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::ops::{Add, AddAssign, Sub};
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
    meta: fs::Metadata,
    //size: String,
    //last_modified: String,
    // TODO metadata
}

struct Directory {
    name: OsString,
    meta: fs::Metadata,
    files: Vec<FileInfo>,
    directories: Vec<Directory>,
}

struct ScrollableArea {
    screen_offset: Vector2,
    size: Vector2,
    curr_pos: Vector2,
}

impl ScrollableArea {
    fn draw(&self, term: &Term) -> io::Result<()> {
        //term.move_cursor_to(self.screen_offset.1 as usize, self.screen_offset.0 as usize)?;

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
            self.draw_tree(dirs, &mut 1, self.root, &mut Vector2 { x: 0, y: 0 });
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

    fn draw_tree(
        &self,
        selected_dirs: &Vec<&'a Directory>,
        selected_dir_num: &mut i32,
        curr_dir: &'a Directory,
        curr_dir_pos: &mut Vector2,
    ) -> io::Result<()> {
        let begin_pos = *curr_dir_pos;

        let curr_dir_name = curr_dir.name.to_str().unwrap();
    
        // convenience closure to prevent repetition
        let print_dir_with_color = |color| print_to_area(self.term, &self.tree_area,
            curr_dir_name, color, begin_pos);

        // print directory with appropriate color
        if selected_dirs.iter().any(|&e| e as *const Directory == curr_dir as *const Directory) {
            print_dir_with_color(Style::new().red())?;
            print_to_area(self.term, &self.tree_area, &format!(": {}", selected_dir_num),
                Style::new().red().bold(), begin_pos + Vector2 { x: curr_dir_name.len(), y: 0 })?;
            *selected_dir_num += 1;
        } else if curr_dir as *const Directory == self.curr_dir as *const Directory {
            print_dir_with_color(Style::new().blue())?;
        } else {
            print_dir_with_color(Style::new().white())?;
        }
    
        let dirs = &curr_dir.directories;
        for (i, dir) in dirs.iter().enumerate() {
            curr_dir_pos.x = begin_pos.x + 3;
            curr_dir_pos.y += 1;
    
            let to_print = if i == dirs.len() - 1 {
                "└─" // U+2514 U+2500
            } else {
                "├─" // U+251C U+2500
            };
    
            print_to_area(self.term, &self.tree_area, to_print, Style::new().white(),
                Vector2 { x: begin_pos.x, y: curr_dir_pos.y })?;
            
            let init_y = begin_pos.y;
            self.draw_tree(selected_dirs, selected_dir_num, dir, curr_dir_pos)?;
    
            for i in init_y+2..curr_dir_pos.y-1 {
                print_to_area(self.term, &self.tree_area, "|", Style::new().white(),
                    Vector2 { x: begin_pos.x, y: i })?;
            }
        }
    
        Ok(())
    }

    fn draw_contents(&self) -> io::Result<()> {
        let term = self.term;
        let area = &self.contents_area;

        // 19 spaces for date
        print_to_area(term, area, "Last Modified           Size  Name", Style::new().white(), Vector2 { x: 1, y: 0 })?;
        print_to_area(term, area, "-------------           ----  ----", Style::new().white(), Vector2 { x: 1, y: 1 })?;
        print_to_area(term, area, "- Directories -",             Style::new().white(), Vector2 { x: 1, y: 2 })?;

        let mut curr_y = 3;
        for dir in &self.curr_dir.directories {
            let last_mod = DateTime::<Utc>::from(dir.meta.modified()?);

            let (pm, hour) = last_mod.hour12();
            print_to_area(term, area, &format!("{:02}/{:02}/{:02} {:02}:{:02} {}           {}",
                last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                dir.name.to_str().unwrap()),                            // file name
                Style::new().white(), Vector2 { x: 1, y: curr_y })?;
            curr_y += 1;
        }

        curr_y += 1;
        print_to_area(term, area, "- Files -", Style::new().white(), Vector2 { x: 1, y: curr_y })?;
        curr_y += 1;

        for file in &self.curr_dir.files {
            let last_mod = DateTime::<Utc>::from(file.meta.modified()?);

            let (pm, hour) = last_mod.hour12();
            print_to_area(term, area, &format!("{:02}/{:02}/{:02} {:02}:{:02} {}  {:>7}  {}",
                last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                file_size_to_str(file.meta.file_size()),                 // file size string
                file.name.to_str().unwrap()),                            // file name
                Style::new().white(), Vector2 { x: 1, y: curr_y })?;
            curr_y += 1;
        }
        Ok(())
    }
}

fn file_size_to_str(size: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if size >= GB {
        format!("{:.0} GB", size as f64 / GB as f64)
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
    let size = {
        let s = term.size();
        Vector2 { x: s.1 as usize, y: s.0 as usize }
    };

    let line_x = (size.x as f64 * 0.6) as usize;
    
    let manager = TerminalManager {
        term: &term,
        root: &root,
        curr_dir: &root,

        tree_area: ScrollableArea {
            screen_offset: Vector2 { x: 1, y: 3 },
            size: Vector2 { x: line_x - 2, y: size.y - 6 },
            curr_pos: Vector2 { x: 0, y: 0 },
        },

        contents_area: ScrollableArea {
            screen_offset: Vector2 { x: line_x + 2, y: 3 },
            size: Vector2 { x: (size.x - line_x) - 2, y: size.y - 6 },
            curr_pos: Vector2 { x: 0, y: 0 },
        },
        
        command_area: ScrollableArea {
            screen_offset: Vector2 { x: 3, y: size.y - 2 },
            size: Vector2 { x: size.x - 2, y: 1 },
            curr_pos: Vector2 { x: 0, y: 0 },
        },
    };

    //draw(&term, &manager);
    for _ in 0..size.y {
        term.write_line("")?;
    }

    let mut curr_area = CurrentArea::Command;
    manager.draw_outline(CurrentArea::Command)?;
    manager.draw_tree(&vec![], &mut 0, &manager.root, &mut Vector2{ x: 0, y: 0 })?;
    manager.draw_contents()?;

    term.move_cursor_to(3, size.y - 1)?;
    
    let mut command = String::new();
    loop {
        let key = term.read_key()?;

        use console::Key::*;
        match key {
            ArrowUp => {
                if let CurrentArea::Command = curr_area {
                    curr_area = CurrentArea::Tree;
                    manager.draw_outline(CurrentArea::Tree)?;
                    manager.term.hide_cursor()?;
                }
            },
            ArrowRight => {
                if let CurrentArea::Tree = curr_area {
                    curr_area = CurrentArea::Contents;
                    manager.draw_outline(CurrentArea::Contents)?;
                }
            },
            ArrowDown | Escape => {
                if curr_area != CurrentArea::Command {
                    curr_area = CurrentArea::Command;
                    manager.draw_outline(CurrentArea::Command)?;
                    manager.term.show_cursor()?;
                }
            },
            ArrowLeft => {
                if let CurrentArea::Contents = curr_area {
                    curr_area = CurrentArea::Tree;
                    manager.draw_outline(CurrentArea::Tree)?;
                }
            },
            Char(c) => {
                match curr_area {
                    CurrentArea::Command => {
                        command.push(c);
                        term.write_str(&c.to_string())?;
                    }
                    _ => {
                        match c {
                            'w' | 'W' => {}, // current area.y -= 1
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
    

    //let abs_path = std::fs::canonicalize(&path)
    //    .expect(&format!("directory {} cannot be found", dir_str));

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
            Direction::Up => ("↑", 1, 0, Vector2 { x: 0, y: 0 }, bounds.size.x),
            Direction::Right => ("→", 0, 1, Vector2 { x: bounds.size.x - 1, y: 0 }, bounds.size.y),
            Direction::Down => ("↓", -1, 0, Vector2 { x: bounds.size.x - 1, y: bounds.size.y }, bounds.size.x),
            Direction::Left => ("←", 0, -1, Vector2 { x: 0, y: bounds.size.y }, bounds.size.y),
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

    // TODO
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
