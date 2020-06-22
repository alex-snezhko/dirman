use std::env;
use std::fs;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::ops::{Add, AddAssign, Sub};
use console::{Term, Key};

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

struct File {
    name: OsString,
    meta: fs::Metadata,
    // TODO metadata
}

struct Directory {
    name: OsString,
    files: Vec<File>,
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

struct TerminalManager<'a> {
    term: Term,
    root: Directory,
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
            "enter" => {},
            "move" => {},
            "rename" => {},
            "copy" => {},
            "new" => {},
            _ => {},
        }
    }

    fn to_directory(&'a self, path: &str) -> Vec<&'a Directory> {
        let parts: Vec<&str> = path.split("/").collect();
        
        let mut possible = vec![&self.root];
        for part in &parts {
            let mut new: Vec<&'a Directory> = Vec::new();
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

    fn move_file(&self, file: &File, old_dir: &Directory, new_dir: &Directory) {
        //fs::
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
    let mut root = Directory {
        name: OsString::from(path),
        files: vec![],
        directories: vec![],
    };
    load_dirs(&mut root, path)?;

    let term = Term::stdout();

    // find dimensions for screen areas
    let size = {
        let s = term.size();
        Vector2 { x: s.1 as usize, y: s.0 as usize }
    };

    let line_x = (size.x as f64 * 0.6) as usize;

    let tree_area = ScrollableArea {
        screen_offset: Vector2 { x: 1, y: 3 },
        size: Vector2 { x: line_x - 2, y: size.y - 6 },
        curr_pos: Vector2 { x: 0, y: 0 },
    };

    let contents_area = ScrollableArea {
        screen_offset: Vector2 { x: line_x + 2, y: 3 },
        size: Vector2 { x: (size.x - line_x) - 2, y: size.y - 6 },
        curr_pos: Vector2 { x: 0, y: 0 },
    };

    let command_area = ScrollableArea {
        screen_offset: Vector2 { x: 3, y: size.y - 2 },
        size: Vector2 { x: size.x - 2, y: 1 },
        curr_pos: Vector2 { x: 0, y: 0 },
    };
    
    let manager = TerminalManager {
        term,
        
        //curr_dir: { &root },
        root,
        tree_area,
        contents_area,
        command_area,
    };

    //draw(&term, &manager);
    draw_outline(size.y, size.x);
    draw_tree(&manager, &manager.root, &mut (manager.tree_area.screen_offset + manager.tree_area.curr_pos))?;

    let term = &manager.term;
    term.move_cursor_to(3, size.y - 1)?;

    //let line = term.read_line()?;
    let mut curr_area = 0;
    let mut command = String::new();
    loop {
        let key = term.read_key()?;

        use console::Key::*;
        match key {
            Escape => {
                curr_area = 1;
            },
            ArrowLeft => {
                if curr_area == 2 {
                    curr_area = 1;
                    // TODO switch(1);
                }
            },
            ArrowRight => {
                if curr_area == 1 {
                    curr_area = 2;
                    // TODO switch(2);
                }
            },
            ArrowDown => {
                if curr_area != 0 {
                    curr_area = 0;
                    // TODO switch(0);
                }
            },
            ArrowUp => {
                if curr_area == 0 {
                    curr_area = 1;
                }
            },
            Char(c) => {
                command.push(c);
                term.write_str(&c.to_string())?;
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

fn load_dirs(curr_dir: &mut Directory, dir_path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;

        let name: OsString = entry.file_name();
        let path: PathBuf = entry.path();
        let meta: fs::Metadata = entry.metadata()?;

        if entry.file_type()?.is_dir() {
            let mut new_dir = Directory {
                name,
                files: vec![],
                directories: vec![],
            };
            load_dirs(&mut new_dir, &path)?;

            curr_dir.directories.push(new_dir);
        } else {
            curr_dir.files.push(File { name, meta });
        }
    }

    Ok(())
}

// fn draw(term: &Term, manager: &TerminalManager) {
    
// }

fn draw_arrows(term: &Term, begin: Vector2, arrow: &str, mov_x: i8, mov_y: i8) -> io::Result<()>
{
    let mut pos = begin;
    term.move_cursor_to(begin.x, begin.y)?;
    for _ in 0..11 {
        term.write_str(arrow)?;
        term.move_cursor_left(1)?;
        pos.x = pos.x.wrapping_add(mov_x as usize);
        pos.y = pos.y.wrapping_add(mov_y as usize);
        //mov()?;
        term.move_cursor_to(pos.x, pos.y)?;
    }
    Ok(())
}

fn print_if_in_bounds(term: &Term, bounds: &ScrollableArea, text: &str, pos: Vector2) -> io::Result<()> {
    let actual_pos = pos - bounds.curr_pos;

    let low_bound = bounds.screen_offset;
    let high_bound = bounds.screen_offset + bounds.size;

    let in_bounds_top = actual_pos.y >= low_bound.y;
    let in_bounds_right = actual_pos.x + text.len() < high_bound.x;
    let in_bounds_bot = actual_pos.y < high_bound.y;
    let in_bounds_left = actual_pos.x >= low_bound.x;
    
    if !in_bounds_top {
        draw_arrows(term, bounds.screen_offset + Vector2 { x: (bounds.size.x / 2) - 5, y: 0 },
            "↑", 1, 0)?;//|| { term.move_cursor_right(1) })?;
    }

    if !in_bounds_right {
        draw_arrows(term, bounds.screen_offset + Vector2 { x: bounds.size.x, y: (bounds.size.y / 2) - 5 },
            "→", 0, 1)?;//|| { term.move_cursor_down(1) })?;
    }

    if !in_bounds_bot {
        draw_arrows(term, bounds.screen_offset + Vector2 { x: (bounds.size.x / 2) + 5, y: bounds.size.y },
            "↓", -1, 0)?;//|| { term.move_cursor_left(1) })?;
    }

    if !in_bounds_left {
        draw_arrows(term, bounds.screen_offset + Vector2 { x: 0, y: (bounds.size.y / 2) + 5 },
            "←", 0, -1)?;//|| { term.move_cursor_up(1) })?;
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
        term.write_str(text)?;
    }

    Ok(())
}

fn draw_tree(manager: &TerminalManager, curr_dir: &Directory, curr_dir_pos: &mut Vector2) -> io::Result<()> {
    let term = &manager.term;
    let tree_area = &manager.tree_area;

    let begin_pos = *curr_dir_pos;

    print_if_in_bounds(term, tree_area, curr_dir.name.to_str().unwrap(), begin_pos)?;

    let dirs = &curr_dir.directories;
    for (i, dir) in dirs.iter().enumerate() {
        curr_dir_pos.x = begin_pos.x + 3;
        curr_dir_pos.y += 1;

        let to_print = if i == dirs.len() - 1 {
            "└─" // U+2514 U+2500
        } else {
            "├─" // U+251C U+2500
        };

        print_if_in_bounds(term, tree_area, to_print, Vector2 { x: begin_pos.x, y: curr_dir_pos.y })?;
        
        let init_y = begin_pos.y;
        draw_tree(manager, dir, curr_dir_pos)?;

        for i in init_y+1..curr_dir_pos.y {
            print_if_in_bounds(term, tree_area, "|", Vector2 { x: begin_pos.x, y: i })?;
        }
    }

    Ok(())
}

fn draw_contents(manager: &TerminalManager, curr_dir: &Directory) -> io::Result<()> {
    Ok(())
}

fn draw_outline(height: usize, width: usize) {
    let line_x = (width as f64 * 0.6) as usize;

    println!("DirMan");
    print!("+");
    for _ in 1..line_x {
        print!("-");
    }
    print!("+");
    for _ in line_x+1..width-1 {
        print!("-");
    }
    println!("+");
    
    for _ in 0..height-4 {
        for _ in 0..line_x {
            print!(" ");
        }
        println!("|");
    }

    print!("+");
    for _ in 1..line_x {
        print!("-");
    }
    print!("+");
    for _ in line_x+1..width-1 {
        print!("-");
    }
    println!("+");
    print!(" > ");
    io::stdout().flush().unwrap();
}
