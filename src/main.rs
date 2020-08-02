use std::env;
use std::fs::{self, File, Metadata};
use std::os::windows::prelude::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::ops::{Add, AddAssign, Sub};
use std::cmp::{PartialEq, max, min};
use console::{Term, Style};
use chrono::{DateTime, Utc, Datelike, Timelike};
use colorful::Color;
use colorful::Colorful;
use colorful::RGB;

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

impl PartialEq for Directory {
    fn eq(&self, other: &Self) -> bool {
        self as *const Directory == other as *const Directory
    }
}

struct ColoredString {
    string: String,
    color: Option<Color>,
}

impl ColoredString {
    fn normal(string: String) -> Self {
        Self { string, color: None }
    }

    fn colored(string: String, color: Color) -> Self {
        Self { string, color: Some(color) }
    }
}

struct ScrollableArea {
    screen_offset: Vector2,  // the location on the terminal window of the top left of this area
    size: Vector2,           // the size of the area
    curr_pos: Vector2,       // current offset into the content
    contents: Vec<Vec<ColoredString>>,   // buffer of all contents that can be printed in this area
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
            if let Some(line) = self.contents.get(self.curr_pos.y + i) {
                term.move_cursor_to(self.screen_offset.x + 2, self.screen_offset.y + 1 + i)?;

                let mut piece_begin = 0;
                for piece in line {
                    
                    let len = piece.string.chars().count();
                    if piece_begin + len < self.curr_pos.x {
                        piece_begin += len;
                        continue;
                    }

                    let skip = match self.curr_pos.x.checked_sub(piece_begin) {
                        Some(i) => i,
                        None => 0,
                    };
                    let take = contents_size.x - ((piece_begin + skip) - self.curr_pos.x);

                    let substr: String = piece.string.chars().skip(skip).take(take).collect();
                    let len = substr.chars().count();

                    term.write_str(&if let Some(color) = &piece.color {
                        format!("{}", substr.color(*color))//color.apply_to(substr))
                    } else {
                        substr
                    })?;

                    piece_begin += skip + len;
                    
                    //term.write_str(&to_print)?;
                }
                
                
                //s.chars().skip(self.curr_pos.x).take(contents_size.x).collect()
            } else {
                break;
            }

            // term.move_cursor_to(self.screen_offset.x + 2, self.screen_offset.y + 1 + i)?;
            // term.write_str(&to_print)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CurrentArea { Command, Tree, Contents }

struct StateManager<'a> {
    term: &'a Term,
    root: &'a Directory,
    curr_dir: &'a Directory,
    hidden_dirs: Vec<&'a Directory>,
    ambiguous_dirs: Vec<&'a Directory>,
    command_buf: Option<&'static str>,
    tree: ScrollableArea,
    dir_contents: ScrollableArea,
}

impl<'a> StateManager<'a> {

    fn process_command(&mut self, command: &str) -> io::Result<()> {
        let tokens: Vec<&str> = command.split_whitespace().collect();
        if tokens.len() == 0 {
            return Ok(());
        }

        let mut command = tokens[0];
        let mut unambiguous_dir = None;
        if let Some(command_buf) = self.command_buf {
            if tokens.len() != 1 {
                print_error(self.term, "Input a single number for disambiguation or 'cancel' to cancel command");
                return Ok(());
            } else {
                if let Ok(num) = tokens[1].parse::<usize>() {
                    command = command_buf;
                    unambiguous_dir = self.ambiguous_dirs.get(num);
                }
                self.command_buf = None;
            }
        }
        
        let get_dir = |command_name, dir_name: Option<&&str>| -> Option<&&Directory> {
            // if an ambiguity has just been resolved then this is the correct directory
            if unambiguous_dir != None {
                unambiguous_dir
            } else {
                // if the possible argument at the position given is valid then check
                if let Some(dir_name) = dir_name {
                    let possible_dirs = to_directory(self.root, dir_name);
                    if possible_dirs.len() > 1 {
                        self.ambiguous_dirs = possible_dirs;
                        self.command_buf = Some(command_name);
                        None
                    } else {
                        possible_dirs.get(0)
                    }
                } else {
                    print_error(self.term, &format!("'{}' command must be followed by a directory", command_name));
                    None
                }
            }
        };

        match command {
            "enter" => {
                if let Some(dir) = get_dir("enter", tokens.get(1)) {
                    self.tree.contents = load_tree(dir, &vec![], &self.hidden_dirs, &mut 0, self.root);
                    self.tree.draw(self.term)?;
                }
                

                // if let Some(dir) = unambiguous_dir {
                //     self.tree.contents = load_tree(dir, &vec![], &self.hidden_dirs, &mut 0, self.root);
                //     self.tree.draw(self.term)?;
                // } else if tokens.len() > 2 {
                //     print_error(self.term, "'enter' command must be followed by a directory");
                // } else if let Some(dir_name) = tokens.get(1) {
                    
                //     let possible_dirs = to_directory(self.root, dir_name);
                //     if possible_dirs.len() > 1 {
                //         self.ambiguous_dirs = possible_dirs;
                //         self.command_buf = (true, "enter");
                //     } else {
                //         self.tree.contents = load_tree(possible_dirs[0], &vec![], &self.hidden_dirs, &mut 0, self.root);
                //         self.tree.draw(self.term)?;
                //     }
                // }
            },
            "open" => {
                if let Some(dir) = get_dir("open", tokens.get(1)) {
                    if let Some(index) = self.hidden_dirs.iter().position(|e| e == dir) {
                        self.hidden_dirs.remove(index);
                        self.tree.contents = load_tree(self.curr_dir, &vec![], &self.hidden_dirs, &mut 0, self.root);
                        self.tree.draw(self.term)?;
                    }
                }
            },
            "close" => {
                if let Some(dir) = get_dir("close", tokens.get(1)) {
                    self.hidden_dirs.push(dir);
                    self.tree.contents = load_tree(self.curr_dir, &vec![], &self.hidden_dirs, &mut 0, self.root);
                    self.tree.draw(self.term)?;
                }
            },
            "move" => { // file   dir
                if let Some(file_name) = tokens.get(1) {
                    if let Some(file) = self.curr_dir.files.iter().find(|e| e.name.to_str().unwrap() == *file_name) {
                        
                    }
                }
                if tokens.len() > 2 {
                    //print_error("'enter' command must be followed by a directory");
                }
            },
            "rename" => {
                if let Some(file_name) = tokens.get(1) {
                    if self.curr_dir.files.iter().any(|e| e.name == *file_name) {
                        if let Some(new_name) = tokens.get(2) {
                            fs::rename(file_name, new_name)?;
                        }
                    }
                }
            },
            "copy" => {
                if let Some(file_name) = tokens.get(1) {
                    if self.curr_dir.files.iter().any(|e| e.name == *file_name) {
                        if let Some(new_name) = tokens.get(2) {
                            fs::copy(file_name, new_name)?;
                        } else {
                            for i in 1.. {
                                let copy_name = PathBuf::from(format!("{}_{}", file_name, i));
                                if copy_name.exists() {
                                    fs::copy(file_name, copy_name)?;
                                }
                            }
                        }
                    }
                }
            },
            "new" => {
                match tokens.get(1) {
                    Some(which) if *which == "file" || *which == "directory" => {
                        if let Some(name) = tokens.get(2) {
                            let name = Path::new(name);
                            if name.exists() {
                                if *which == "file" {
                                    File::create(name)?;
                                    // TODO place in correct directory and make sure it gets created
                                } else {
                                    fs::create_dir(name)?;
                                    self.tree.contents = load_tree(self.curr_dir, &vec![], &self.hidden_dirs, &mut 0, self.root);
                                    self.tree.draw(self.term)?;
                                }
                                self.dir_contents.contents = load_contents(self.curr_dir);
                                self.dir_contents.draw(self.term)?;
                            } else {
                                print_error(self.term, "File or directory with this name already exists")?;
                            }
                        }
                        print_error(self.term, "Expected name of new file or directory")?;
                    },
                    _ => {
                        print_error(self.term, "Expected 'file' or 'directory' after 'new'")?;
                    }
                }
            },
            _ => {},
        }

        self.term.move_cursor_to(3, self.term.size().0 as usize - 1)?;
        Ok(())
    }

    fn open(&self, args: &[Vec<&str>]) {
        
    }

    fn highlight_if_multiple(&self, dirs: &Vec<&'a Directory>) -> io::Result<()> {
        if dirs.len() > 1 {
            load_tree(self.curr_dir, &vec![], dirs, &mut 0, self.root);
            self.tree.draw(self.term)?;
        }
        self.term.move_cursor_to(0, 0)?;
        //print_error(self.term.write_str("Ambiguous file/directory; input number of intended choice")?;

        Ok(())
    }

}

fn print_error(term: &Term, message: &str) -> io::Result<()> {
    term.move_cursor_to(0, 0)?;
    term.write_str(&format!("{}", message.color(Color::Red)))?;
    Ok(())
}

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

fn draw_outline(term: &Term, selected_panel: CurrentArea) -> io::Result<()> {
    let (height, width) = {
        let size = term.size();
        (size.0 as usize, size.1 as usize)
    };

    let line_x = (width as f64 * 0.5) as usize;

    term.move_cursor_to(0, 0)?;
    term.write_line("DirMan")?;

    let red = Style::new().red();
    let print_with_color = |text: &str, colored_list: Vec<CurrentArea>| -> io::Result<()> {
        let colored = if colored_list.contains(&selected_panel) {
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
    term.move_cursor_down(1)?;
    
    for _ in 0..height-4 {
        term.move_cursor_right(line_x)?;
        print_with_color("┃", vec![Tree, Contents])?;
        term.move_cursor_down(1)?;
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
    ambiguous_dirs: &Vec<&Directory>,
    hidden_dirs: &Vec<&Directory>,
    selected_dir_num: &mut i32, // counter for when multiple directories are selected, to display each with own number
    curr_dir: &Directory,    // current dir being processed
) -> Vec<Vec<ColoredString>>
{
    let mut contents = vec![];

    let curr_dir_name: String = curr_dir.name.clone().into_string().unwrap();

    let mut hidden = false;
    // print item with correct color
    contents.push(vec![
        if ambiguous_dirs.iter().any(|&e| e == curr_dir) {   // red
            let text = format!("{}: {}", curr_dir_name, selected_dir_num);
            *selected_dir_num += 1;
            ColoredString::colored(text, Color::Red)
        } else if hidden_dirs.iter().any(|&e| e == curr_dir) {   // gray
            hidden = true;
            ColoredString::colored(format!("{} +", curr_dir_name), Color::DarkGray)
        } else if curr_dir == selected_dir {   // blue
            ColoredString::colored(curr_dir_name, Color::Blue)
        } else {   // normal
            ColoredString::normal(curr_dir_name)
        }]);

    if hidden {
        return contents;
    }

    // iterate through all child directories and load them as well
    let dirs = &curr_dir.directories;
    for (i, dir) in dirs.iter().enumerate() {
        // next index will need pipe if it is not the last one in branch
        let (begin, next) = if i == dirs.len() - 1 {
            ("└─ ", "   ")
        } else {
            ("├─ ", "│  ")
        };

        let inner_dir_content = load_tree(selected_dir, ambiguous_dirs, hidden_dirs, selected_dir_num, dir);

        let mut first = true;
        for e in inner_dir_content {
            let mut line = vec![ColoredString::normal(if first { first = false; begin } else { next }.to_string())];
            for piece in e {
                line.push(piece);
            }
            contents.push(line);
        }
    }

    contents
}

fn load_contents(curr_dir: &Directory) -> Vec<Vec<ColoredString>> {
    let mut contents = vec![];

    contents.push(vec![ColoredString::normal("Last Modified           Size  Name".to_string())]);
    contents.push(vec![ColoredString::normal("-------------           ----  ----".to_string())]);
    if curr_dir.directories.len() != 0 {
        contents.push(vec![ColoredString::normal("- Directories -".to_string())]);

        for dir in &curr_dir.directories {
            let last_mod = DateTime::<Utc>::from(dir.meta.modified().unwrap());

            let (pm, hour) = last_mod.hour12();
            contents.push(vec![ColoredString::normal(format!("{:02}/{:02}/{:02} {:02}:{:02} {}           {}",
                last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                dir.name.to_str().unwrap()))]);                            // file name
        }
        contents.push(vec![ColoredString::normal(String::new())]);
    }

    if curr_dir.files.len() != 0 {
        contents.push(vec![ColoredString::normal("- Files -".to_string())]);

        for file in &curr_dir.files {
            let last_mod = DateTime::<Utc>::from(file.meta.modified().unwrap());

            let (pm, hour) = last_mod.hour12();
            contents.push(vec![ColoredString::normal(format!("{:02}/{:02}/{:02} {:02}:{:02} {}  {:>7}  {}",
                last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                file_size_to_str(file.meta.file_size()),                 // file size string
                file.name.to_str().unwrap()))]);                           // file name
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

    let line_x = (size.x as f64 * 0.5) as usize;

    let tree_contents = load_tree(&root, &vec![], &vec![], &mut 0, &root);
    let tree_area = ScrollableArea {
        screen_offset: Vector2 { x: 0, y: 2 },
        size: Vector2 { x: line_x, y: size.y - 4 },
        curr_pos: Vector2 { x: 0, y: 0 },
        longest_line_len: tree_contents.iter()
            .fold(0, |largest, line| max(largest, line.iter()
                .fold(0, |len, piece| len + piece.string.chars().count()))),
        contents: tree_contents,
    };

    let dir_contents = load_contents(&root);
    let contents_area = ScrollableArea {
        screen_offset: Vector2 { x: line_x + 1, y: 2 },
        size: Vector2 { x: (size.x - line_x - 1), y: size.y - 4 },
        curr_pos: Vector2 { x: 0, y: 0 },
        longest_line_len: dir_contents.iter()
            .fold(0, |largest, line| max(largest, line.iter()
                .fold(0, |acc, piece| acc + piece.string.chars().count()))),
        contents: dir_contents,
    };

    let command_area = ScrollableArea {
        screen_offset: Vector2 { x: 3, y: size.y - 2 },
        size: Vector2 { x: size.x - 3, y: 1 },
        curr_pos: Vector2 { x: 0, y: 0 },
        contents: vec![],
        longest_line_len: 0,
    };

    let mut manager = StateManager {
        term: &term,
        root: &root,
        curr_dir: &root,
        hidden_dirs: vec![],
        ambiguous_dirs: vec![],
        command_buf: None,
        tree: tree_area,
        dir_contents: contents_area,
    };

    for _ in 0..size.y {
        term.write_line("")?;
    }

    draw_outline(&term, CurrentArea::Command)?;

    manager.tree.draw(&term)?;
    manager.dir_contents.draw(&term)?;

    term.move_cursor_to(3, size.y - 1)?;

    let mut curr_area_tag = CurrentArea::Command;
    
    let mut command = String::new();
    loop {
        let key = term.read_key()?;

        // TODO handle resize https://docs.rs/crossterm/0.17.7/crossterm/event/fn.poll.html
        use console::Key::*;
        match key {
            ArrowUp => {
                if let CurrentArea::Command = curr_area_tag {
                    curr_area_tag = CurrentArea::Tree;
                    draw_outline(&term, CurrentArea::Tree)?;
                    term.hide_cursor()?;
                }
            },
            ArrowRight => {
                if let CurrentArea::Tree = curr_area_tag {
                    curr_area_tag = CurrentArea::Contents;
                    draw_outline(&term, CurrentArea::Contents)?;
                }
            },
            ArrowDown | Escape => {
                if curr_area_tag != CurrentArea::Command {
                    curr_area_tag = CurrentArea::Command;
                    draw_outline(&term, CurrentArea::Command)?;
                    term.show_cursor()?;
                }
            },
            ArrowLeft => {
                if let CurrentArea::Contents = curr_area_tag {
                    curr_area_tag = CurrentArea::Tree;
                    draw_outline(&term, CurrentArea::Tree)?;
                }
            },
            Char(c) => {
                if let CurrentArea::Command = curr_area_tag {
                    command.push(c);
                    term.write_str(&c.to_string())?;
                } else {
                    let curr_area = match curr_area_tag {
                        CurrentArea::Tree => &mut manager.tree,
                        CurrentArea::Contents => &mut manager.dir_contents,
                        _ => &mut manager.tree,
                    };

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
                manager.process_command(&command)?;
                let chars = command.chars().count();
                term.move_cursor_right(chars)?;
                term.clear_chars(command.chars().count())?;
                command.clear();
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
