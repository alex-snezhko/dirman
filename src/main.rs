use std::env;
use std::fs::{self, Metadata};
use std::os::windows::prelude::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::ops::{Add, AddAssign, Sub};
use std::cmp::{PartialEq, max, min};
use std::cell::RefCell;
use std::rc::Rc;
use console::Term;
use chrono::{DateTime, Utc, Datelike, Timelike};
use colorful::Color;
use colorful::Colorful;

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
    meta: Metadata,
    full_path: PathBuf,
}

struct Directory {
    name: OsString,
    meta: Metadata,
    full_path: PathBuf,
    files: Vec<FileRef>,
    directories: Vec<DirectoryRef>,
    parent: Option<DirectoryRef>,
}

impl File {
    fn new(path: PathBuf) -> Self {
        Self {
            name: OsString::from(path.file_name().unwrap()),
            meta: path.metadata().unwrap(),
            full_path: path,
        }
    }
}

impl Directory {
    fn new(
        path: PathBuf,
        files: Vec<FileRef>,
        directories: Vec<DirectoryRef>,
        parent: Option<DirectoryRef>
    ) -> Self
    {
        Self {
            name: OsString::from(path.file_name().unwrap()),
            meta: path.metadata().unwrap(),
            full_path: path,
            files,
            directories,
            parent,
        }
    }
}

type DirectoryRef = Rc<RefCell<Directory>>;
type FileRef = Rc<RefCell<File>>;

impl PartialEq for Directory {
    fn eq(&self, other: &Self) -> bool {
        self as *const Self == other as *const Self
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

            const X_EVERY: usize = 5;
            const Y_EVERY: usize = 3;

            use ArrowLocation::*;
            // begin_offset determines where to place the terminal cursor for the first arrow
            // in the sequence; it is calculated by the principle that as many arrows should be
            // drawn as can fit in the bounds; the complex calculation involves finding the size of
            // the "line" of arrows to be drawn (including one extra as fence-post) and centering it
            let (arrow, horizontal, begin_offset, count) = match direction {
                Top => ("↑", true, Vector2 { x: 1 + (s.x - (((s.x - 1) / X_EVERY) * X_EVERY + 1)) / 2, y: 0 }, (s.x - 1) / X_EVERY + 1),
                Right => ("→", false, Vector2 { x: s.x + 1, y: 1 + (s.y - (((s.y - 1) / Y_EVERY) * Y_EVERY + 1)) / 2 }, (s.y - 1) / Y_EVERY + 1),
                Bottom =>  ("↓", true, Vector2 { x: 1 + (s.x - (((s.x - 1) / X_EVERY) * X_EVERY + 1)) / 2, y: s.y + 1 }, (s.x - 1) / X_EVERY + 1),
                Left => ("←", false, Vector2 { x: 0, y: 1 + (s.y - (((s.y - 1) / Y_EVERY) * Y_EVERY + 1)) / 2 }, (s.y - 1) / Y_EVERY + 1),
            };

            let mut pos = self.screen_offset + begin_offset;
            for _ in 0..count {
                term.move_cursor_to(pos.x, pos.y)?;
                term.write_str(&format!("{}", arrow.color(Color::DarkGray)))?;

                if horizontal {
                    pos.x += X_EVERY;
                } else {
                    pos.y += Y_EVERY;
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

        // prints all of the contents that can fit in the area
        for i in 0..contents_size.y {
            if let Some(line) = self.contents.get(self.curr_pos.y + i) {
                term.move_cursor_to(self.screen_offset.x + 2, self.screen_offset.y + 1 + i)?;

                // some lines are split apart e.g. |└─ |dir1|; make sure the correct number
                // of characters on each line are printed by keeping counter of total characters
                // printed so far and going up to width of the screen
                let mut piece_begin = 0;
                for piece in line {
                    
                    let len = piece.string.chars().count();
                    if piece_begin + len < self.curr_pos.x {
                        piece_begin += len;
                        continue;
                    }

                    // how many characters in the line to skip based on current screen position
                    let skip = match self.curr_pos.x.checked_sub(piece_begin) {
                        Some(i) => i,
                        None => 0,
                    };
                    // how many characters in total are needed
                    let take = contents_size.x - ((piece_begin + skip) - self.curr_pos.x);

                    // get the string of how many characters 
                    let substr: String = piece.string.chars().skip(skip).take(take).collect();
                    let len = substr.chars().count();

                    // print in color if needed
                    term.write_str(&if let Some(color) = &piece.color {
                        format!("{}", substr.color(*color))
                    } else {
                        substr
                    })?;

                    // advance number of characters from this piece that were printed
                    piece_begin += skip + len;
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CurrentArea { Command, Tree, Contents }

// used with command buffering for finding the appropriate directory in the directory tree as queried
enum DirQuery {
    Disambiguated(DirectoryRef),
    ByName(String),
}

// a function that will be used for command buffering if a directory ambiguity is present
type CommandProcedure<'a> = fn(&mut StateManager<'a>, DirQuery, &str) -> io::Result<()>;

// object which manages 'global' state of the program
struct StateManager<'a> {
    term: &'a Term,
    root: DirectoryRef,
    curr_dir: DirectoryRef,
    closed_dirs: Vec<DirectoryRef>,
    ambiguous_dirs: Vec<DirectoryRef>,
    command_buf: Option<(CommandProcedure<'a>, String)>,
    error_message_active: bool,
    dir_to_remove: Option<DirectoryRef>,
    tree: ScrollableArea,
    dir_contents: ScrollableArea,
}

impl<'a> StateManager<'a> {
    fn init(term: &'a Term, root: DirectoryRef) -> io::Result<Self> {
        let term_size = Vector2 { x: term.size().1 as usize, y: term.size().0 as usize };
        let line_x = (term_size.x as f64 * 0.5) as usize;

        let tree_area = ScrollableArea {
            screen_offset: Vector2 { x: 0, y: 2 },
            size: Vector2 { x: line_x, y: term_size.y - 4 },
            curr_pos: Vector2 { x: 0, y: 0 },
            longest_line_len: 0,
            contents: vec![],
        };

        let contents_area = ScrollableArea {
            screen_offset: Vector2 { x: line_x + 1, y: 2 },
            size: Vector2 { x: (term_size.x - line_x - 1), y: term_size.y - 4 },
            curr_pos: Vector2 { x: 0, y: 0 },
            longest_line_len: 0,
            contents: vec![],
        };

        let mut new = Self {
            term,
            root: root.clone(),
            curr_dir: root.clone(),
            closed_dirs: vec![],
            ambiguous_dirs: vec![],
            command_buf: None,
            error_message_active: false,
            dir_to_remove: None,
            tree: tree_area,
            dir_contents: contents_area,
        };

        new.refresh_area(true, true)?;

        Ok(new)
    }

    // processes a user command and updates the directory contents if needed
    fn process_command(&mut self, command: &str) -> io::Result<()> {
        let tokens: Vec<&str> = command.split_whitespace().collect();
        if tokens.len() == 0 {
            self.print_error("Enter a command")?;
            return Ok(());
        }

        // initially assume no error; if no error message is printed from this command then clear
        // the error message (if there is one)
        self.error_message_active = false;

        if self.dir_to_remove.is_some() {
            if tokens.len() == 1 {
                if tokens[0] == "yes" {
                    self.clear_error()?;
                    self.remove_dir(DirQuery::Disambiguated(self.dir_to_remove.clone().unwrap()), "")?;
                    return Ok(());
                } else if tokens[0] == "no" {
                    self.clear_error()?;
                    self.command_buf = None;
                    self.dir_to_remove = None;
                    self.refresh_area(true, false)?;
                    return Ok(());
                }
            }
            self.print_error("Input either 'yes' or 'no' for removal of directory and its contents")?;
        }

        // this condition is true if there is disambiguation needed
        if self.command_buf.is_some() {
            // expect a single number for disambiguation or 'cancel'
            if tokens.len() == 1 {
                if let Ok(num) = tokens[0].parse::<usize>() {
                    if self.ambiguous_dirs.get(num).is_some() && self.command_buf.is_some() {
                        self.clear_error()?;

                        let command = self.command_buf.as_ref().unwrap();
                        let unambiguous_dir = self.ambiguous_dirs[num].clone();
                        let command_string = command.1.clone();
                        
                        self.ambiguous_dirs.clear();

                        // execute the buffered command with the now disambiguated directory
                        command.0(self, DirQuery::Disambiguated(unambiguous_dir), &command_string)?;
                        self.command_buf = None;
                        return Ok(());
                    }
                } else if tokens[0] == "cancel" {
                    self.clear_error()?;
                    self.ambiguous_dirs.clear();
                    self.command_buf = None;
                    self.refresh_area(true, false)?;
                    return Ok(());
                }
            }

            self.print_error("Input a number for disambiguation or 'cancel' to cancel command")?;
            return Ok(());
        }

        match tokens[0] {
            // enter <directory>
            "enter" => {
                if tokens.len() == 2 {
                    self.enter_dir(DirQuery::ByName(tokens[1].to_string()), "")?;
                } else {
                    self.print_error("Usage: enter <directory>")?;
                }
            },

            // open <directory>
            "open" => {
                if tokens.len() == 2 {
                    self.open_dir(DirQuery::ByName(tokens[1].to_string()), "")?;
                } else {
                    self.print_error("Usage: open <directory>")?;
                }
            },

            // close <directory>
            "close" => {
                if tokens.len() == 2 {
                    self.close_dir(DirQuery::ByName(tokens[1].to_string()), "")?;
                } else {
                    self.print_error("Usage: close <directory>")?;
                }
            },

            // move <file> <directory>
            "move" => {
                if tokens.len() == 3 {
                    if self.curr_dir.borrow().files.iter().any(|e| e.borrow().name == tokens[1]) {
                        self.move_to_dir(DirQuery::ByName(tokens[2].to_string()), tokens[1])?;
                    } else {
                        self.print_error("File attempted to be moved does not exist")?;
                    }
                } else {
                    self.print_error("Usage: move <file> <directory>")?;
                }
            },

            // copy <file> <directory>
            "copy" => {
                if tokens.len() == 3 {
                    if self.curr_dir.borrow().files.iter().any(|e| e.borrow().name == tokens[1]) {
                        self.copy_to_dir(DirQuery::ByName(tokens[2].to_string()), tokens[1])?;
                    } else {
                        self.print_error("File attempted to be copied does not exist")?;
                    }
                } else {
                    self.print_error("Usage: copy <file> <directory>")?;
                }
            },

            // rename <file|directory> <new_name>
            "rename" => {
                if tokens.len() == 3 {
                    let maybe_old_file_index: Option<usize> = self.curr_dir
                        .clone()
                        .borrow()
                        .files.iter()
                        .position(|e| e.borrow().name == tokens[1]);

                    if let Some(old_file_index) = maybe_old_file_index {
                        let mut new_path = self.curr_dir.borrow().full_path.clone();
                        new_path.push(tokens[2]);

                        fs::rename(&self.curr_dir.borrow().files[old_file_index].borrow().full_path, &new_path)?;

                        // remove the file from the tree and add the new one
                        self.curr_dir.borrow_mut().files.remove(old_file_index);
                        Self::add_item_to_tree(self.curr_dir.clone(), new_path);

                        self.refresh_area(false, true)?;
                    } else {
                        self.rename_dir(DirQuery::ByName(tokens[1].to_string()), tokens[2])?;
                    }
                } else {
                    self.print_error("Usage: rename <file|directory> <new_name>")?;
                }
            },

            // new [file|directory] <name>
            // creates a new file or directory in the current directory
            "new" => {
                if tokens.len() == 3 {
                    let which = tokens[1];
                    if which == "file" || which == "directory" {
                        let mut new_path = self.curr_dir.borrow().full_path.clone();
                        new_path.push(tokens[2]);
                        if !new_path.exists() {
                            if which == "file" {
                                fs::File::create(&new_path)?;
                                Self::add_item_to_tree(self.curr_dir.clone(), new_path);
                            } else {
                                fs::create_dir(&new_path)?;
                                let new_dir = Rc::new(RefCell::new(
                                    Directory::new(new_path, vec![], vec![], Some(self.curr_dir.clone())))
                                );

                                self.curr_dir.borrow_mut().directories.push(new_dir);
                                self.curr_dir.borrow_mut().directories.sort_by(|x, y| x.borrow().name.cmp(&y.borrow().name));

                                self.refresh_area(true, false)?;
                            }
                            self.refresh_area(false, true)?;
                        } else {
                            self.print_error("File or directory with this name already exists")?;
                        }
                    }
                } else {
                    self.print_error("Usage: new [file|directory] <name>")?;
                }
            },

            // remove <file|directory>
            "remove" => {
                if tokens.len() == 2 {
                    let maybe_file_index = self.curr_dir.borrow().files.iter().position(|e| e.borrow().name == tokens[1]);
                    if let Some(file_index) = maybe_file_index {
                        fs::remove_file(&self.curr_dir.borrow().files[file_index].borrow().full_path)?;
                        self.curr_dir.borrow_mut().files.remove(file_index);
                        self.refresh_area(false, true)?;
                    } else {
                        self.remove_dir(DirQuery::ByName(tokens[1].to_string()), "")?;
                    }
                } else {
                    self.print_error("Usage: remove <file|directory>")?;
                }
            }

            _ => {
                self.print_error("Invalid command")?;
            },
        }

        if !self.error_message_active {
            self.clear_error()?;
        }
        
        Ok(())
    }

    // gets path for new file (copied, moved, etc); will be different from original if a file
    // with the given path already exists
    fn get_copy_path(original: PathBuf) -> PathBuf {
        if original.exists() {
            for i in 1.. {
                let mut s = String::new();
                if let Some(file_stem) = original.file_stem() {
                    s.push_str(file_stem.to_str().unwrap());
                }
                s.push_str(&format!("_{}", i));
                if let Some(ext) = original.extension() {
                    s.push_str(&format!(".{}", ext.to_str().unwrap()));
                }

                let mut copy = original.parent().unwrap().to_path_buf();
                copy.push(s);
                if !copy.exists() {
                    return copy;
                }
            }
        }
        original
    }

    // adds a file to the in-memory directory tree data structure
    fn add_item_to_tree(dir: DirectoryRef, path: PathBuf) {
        // files/dirs are kept sorted
        let files = &mut dir.borrow_mut().files;
        files.push(Rc::new(RefCell::new(File::new(path))));
        files.sort_by(|x, y| x.borrow().name.cmp(&y.borrow().name));
    }

    // +----------------------------------+
    // |   Bufferable command functions   |
    // +----------------------------------+

    // helper function for *_dir methods below; checks dir parameter and returns a concrete
    // Directory if there is no ambiguity or None if there is
    fn get_dir(&mut self, func: CommandProcedure<'a>, dir_query: DirQuery, other_arg: &str)
        -> io::Result<Option<DirectoryRef>>
    {
        match dir_query {
            // if this directory has been already disambiguated then return it
            DirQuery::Disambiguated(dir) => Ok(Some(dir)),
            // if this directory is queried by name then try to find it or return None if ambiguous
            DirQuery::ByName(dir_name) => {
                // get list of all possible directories that match the query
                let possible_dirs = self.to_directory(&dir_name);
                if possible_dirs.len() > 1 {
                    self.print_error("Ambiguous directory; input number corresponding to intended choice")?;
                    self.ambiguous_dirs = possible_dirs;
                    self.refresh_area(true, false)?;
                    // buffer a command for disambiguation
                    self.command_buf = Some((func, other_arg.to_string()));
                    Ok(None)
                } else {
                    // if directory is unambiguous return it
                    if let Some(d) = possible_dirs.get(0) {
                        Ok(Some(d.clone()))
                    } else {
                        self.print_error("Specified directory does not exist")?;
                        Ok(None)
                    }
                }
            }
        }
    }

    // enter a directory to view its contents
    fn enter_dir(&mut self, dir: DirQuery, other_arg: &str) -> io::Result<()> {
        if let Some(dir) = self.get_dir(Self::enter_dir, dir, other_arg)? {
            self.curr_dir = dir;
            self.refresh_area(true, true)?;
        }
        Ok(())
    }

    // hides the inner directories of an opened directory in the directory tree
    fn close_dir(&mut self, dir: DirQuery, other_arg: &str) -> io::Result<()> {
        if let Some(dir) = self.get_dir(Self::close_dir, dir, other_arg)? {
            self.closed_dirs.push(dir.clone());

            // if the current directory is a child of the closed directory then move
            // the current directory up to the closed one
            if self.curr_dir.borrow().full_path.to_str().unwrap()
               .contains(dir.borrow().full_path.to_str().unwrap())
            {
                self.curr_dir = dir;
            }

            self.refresh_area(true, true)?;
        }
        Ok(())
    }

    // opens a closed directory in the directory tree
    fn open_dir(&mut self, dir: DirQuery, other_arg: &str) -> io::Result<()> {
        if let Some(dir) = self.get_dir(Self::open_dir, dir, other_arg)? {
            if let Some(index) = self.closed_dirs.iter().position(|e| *e == dir) {
                self.closed_dirs.remove(index);
                self.refresh_area(true, false)?;
            }
        }
        Ok(())
    }

    // moves a file into a different directory
    fn move_to_dir(&mut self, dir: DirQuery, file_name: &str) -> io::Result<()> {
        if let Some(dir) = self.get_dir(Self::move_to_dir, dir, file_name)? {
            let mut file_path = self.curr_dir.borrow().full_path.clone();
            file_path.push(file_name);
            let mut new_path = dir.borrow().full_path.clone();
            new_path.push(file_name);
            new_path = Self::get_copy_path(new_path);
            fs::rename(file_path, &new_path)?;

            // remove this file from the current directory
            let index = self.curr_dir.borrow().files
                .iter()
                .position(|e| e.borrow().name == file_name)
                .unwrap();
            self.curr_dir.borrow_mut().files.remove(index);

            // add this file to its new directory
            Self::add_item_to_tree(dir, new_path);
            self.refresh_area(false, true)?;
        }

        Ok(())
    }

    // copies a file to a different directory
    fn copy_to_dir(&mut self, dir: DirQuery, file_name: &str) -> io::Result<()> {
        if let Some(dir) = self.get_dir(Self::move_to_dir, dir, file_name)? {
            let mut file_path = self.curr_dir.borrow().full_path.clone();
            file_path.push(file_name);
            let mut new_path = dir.borrow().full_path.clone();
            new_path.push(file_name);
            new_path = Self::get_copy_path(new_path);
            fs::copy(file_path, &new_path)?;

            // add this file to its new directory
            Self::add_item_to_tree(dir, new_path);
            self.refresh_area(false, true)?;
        }

        Ok(())
    }

    // renames a directory
    fn rename_dir(&mut self, dir: DirQuery, new_name: &str) -> io::Result<()> {
        if let Some(dir) = self.get_dir(Self::rename_dir, dir, new_name)? {
            // cannot rename root directory
            if dir.borrow().parent.is_none() {
                self.print_error("Cannot rename root directory")?;
                return Ok(());
            }

            let mut new_path = dir.borrow().full_path.parent().unwrap().to_path_buf();
            new_path.push(new_name);

            fs::rename(&dir.borrow().full_path, &new_path)?;

            dir.borrow_mut().name = OsString::from(new_name);
            dir.borrow_mut().full_path = new_path;

            // re-sort directories
            if let Some(parent) = &dir.borrow().parent {
                parent.borrow_mut().directories.sort_by(|x, y| x.borrow().name.cmp(&y.borrow().name));
            }

            self.refresh_area(true, true)?;
        }

        Ok(())
    }

    // removes a directory
    fn remove_dir(&mut self, dir: DirQuery, other_arg: &str) -> io::Result<()> {
        if let Some(dir) = self.get_dir(Self::remove_dir, dir, other_arg)? {
            // cannot remove root directory
            if dir.borrow().parent.is_none() {
                self.print_error("Cannot remove root directory")?;
                return Ok(());
            }

            if self.dir_to_remove.is_some() {
                // dir_to_remove is just be dir at this point
                fs::remove_dir_all(&dir.borrow().full_path)?;

                let borrow = dir.borrow();
                let parent = borrow.parent.as_ref().unwrap();

                let index_to_remove = parent.borrow().directories.iter().position(|e| e == &dir).unwrap();

                // if the current directory is a child of the closed directory then move
                // the current directory up to the closed one
                if self.curr_dir.borrow().full_path.to_str().unwrap()
                   .contains(dir.borrow().full_path.to_str().unwrap())
                {
                    self.curr_dir = parent.clone();
                }

                self.dir_to_remove = None;

                parent.borrow_mut().directories.remove(index_to_remove);
                self.refresh_area(true, true)?;
            } else {
                self.print_error("The directory and all of its contents will be removed. Continue? yes/no")?;
                self.dir_to_remove = Some(dir);
            }

            self.refresh_area(true, true)?;
        }

        Ok(())
    }

    // +-----------------------------------------+
    // |   End of bufferable command functions   |
    // +-----------------------------------------+

    // prints an error message to the top of the terminal window
    fn print_error(&mut self, message: &str) -> io::Result<()> {
        self.term.move_cursor_to(0, 0)?;
        self.term.clear_line()?;
        self.term.write_str(&format!("{}", message.color(Color::Red)))?;
        self.error_message_active = true;
        Ok(())
    }
    
    // clears error message (if there was one) and prints 'DirMan' at top of terminal window
    fn clear_error(&self) -> io::Result<()> {
        self.term.move_cursor_to(0, 0)?;
        self.term.clear_line()?;
        self.term.write_str("DirMan")?;
        Ok(())
    }

    fn load_tree_contents(&self,
        selected_dir_num: &mut i32, // counter for when multiple directories are selected, to display each with own number
        curr_dir: DirectoryRef,     // current dir being processed
    ) -> Vec<Vec<ColoredString>>
    {
        let mut contents = vec![];
    
        let curr_dir_name: String = curr_dir.borrow().name.clone().into_string().unwrap();
    
        // flags for what needs to be printed e.g. '<dir> +' and/or '<dir>: #' for closed/ambiguous
        let mut remove = false;
        let mut closed = false;
        let mut ambiguous = false;

        // flag for which color to print the directory name in;
        // precedence: current(blue) > remove(red) > ambiguous(green) > closed(gray) > normal(default color)
        let mut dir_name_color: Option<Color> = None;
        
        if curr_dir == self.curr_dir {
            dir_name_color = Some(Color::Blue);
        }
        if self.dir_to_remove.is_some() && self.dir_to_remove.as_ref().unwrap() == &curr_dir {
            remove = true;
            if let None = dir_name_color {
                dir_name_color = Some(Color::Red);
            }
        }
        if self.ambiguous_dirs.contains(&curr_dir) {
            ambiguous = true;
            if let None = dir_name_color {
                dir_name_color = Some(Color::Green);
            }
        }
        if self.closed_dirs.contains(&curr_dir) {
            closed = true;
            if let None = dir_name_color {
                dir_name_color = Some(Color::DarkGray);
            }
        }

        // append all text related to the directory name
        let mut directory_text = vec![];
        directory_text.push(
            if let Some(color) = dir_name_color { ColoredString::colored(curr_dir_name, color) }
            else { ColoredString::normal(curr_dir_name) }
        );
        if remove {
            directory_text.push(ColoredString::colored(String::from(" x"), Color::Red));
        }
        if ambiguous {
            directory_text.push(ColoredString::colored(format!(": {}", selected_dir_num), Color::Green));
            *selected_dir_num += 1;
        }
        if closed {
            directory_text.push(ColoredString::colored(String::from(" +"), Color::DarkGray));
        }
        contents.push(directory_text);
    
        // do not continue deeper into tree if this directory is hidden
        if closed {
            return contents;
        }
    
        // iterate through all child directories and load them as well
        let dirs = &curr_dir.borrow().directories;
        for (i, dir) in dirs.iter().enumerate() {
            // next index will need pipe if it is not the last one in branch
            let (begin, next) = if i == dirs.len() - 1 {
                ("└─ ", "   ")
            } else {
                ("├─ ", "│  ")
            };
    
            let inner_dir_content = self.load_tree_contents(selected_dir_num, dir.clone());
    
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

    fn load_dir_contents(&self) -> Vec<Vec<ColoredString>> {
        let mut contents = vec![];

        let files = &self.curr_dir.borrow().files;
        let dirs = &self.curr_dir.borrow().directories;

        if files.len() == 0 && dirs.len() == 0 {
            contents.push(vec![ColoredString::normal("This directory is empty".to_string())]);
        } else {
            contents.push(vec![ColoredString::colored("Last Modified           Size  Name".to_string(), Color::Cyan)]);
            contents.push(vec![ColoredString::colored("‾‾‾‾‾‾‾‾‾‾‾‾‾           ‾‾‾‾  ‾‾‾‾".to_string(), Color::Cyan)]);
        }

        if files.len() != 0 {
            contents.push(vec![ColoredString::normal("- Files -".to_string())]);
    
            for file in files {
                let last_mod = DateTime::<Utc>::from(file.borrow().meta.modified().unwrap());
    
                let (pm, hour) = last_mod.hour12();
                contents.push(vec![ColoredString::normal(format!("{:02}/{:02}/{:02} {:02}:{:02} {}  {:>7}  {}",
                    last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                    hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                    file_size_to_str(file.borrow().meta.file_size()),                 // file size string
                    file.borrow().name.to_str().unwrap()))]);                           // file name
            }
            contents.push(vec![ColoredString::normal(String::new())]);
        }

        if dirs.len() != 0 {
            contents.push(vec![ColoredString::normal("- Directories -".to_string())]);
    
            for dir in dirs {
                let last_mod = DateTime::<Utc>::from(dir.borrow().meta.modified().unwrap());
    
                let (pm, hour) = last_mod.hour12();
                contents.push(vec![ColoredString::normal(format!("{:02}/{:02}/{:02} {:02}:{:02} {}           {}",
                    last_mod.month(), last_mod.day(), last_mod.year(),       // last modified date
                    hour, last_mod.minute(), if pm { "PM" } else { "AM" },   // last modified time
                    dir.borrow().name.to_str().unwrap()))]);                            // file name
            }
            
        }
    
        contents
    }

    // reloads contents of (and redraws) the specified areas
    fn refresh_area(&mut self, tree: bool, contents: bool) -> io::Result<()> {
        fn refresh(term: &Term, area: &mut ScrollableArea, new_contents: Vec<Vec<ColoredString>>) -> io::Result<()> {
            let new_width = new_contents.iter()
                .fold(0, |largest, line| max(largest, line.iter()
                    .fold(0, |len, piece| len + piece.string.chars().count())));
            let new_height = new_contents.len();

            let farthest_right = area.curr_pos.x + area.size.x;
            if farthest_right > new_width {
                area.curr_pos.x -= farthest_right - new_width;
            }

            let farthest_down = area.curr_pos.y + area.size.y;
            if farthest_down > new_height {
                area.curr_pos.x -= farthest_down - new_height;
            }

            area.longest_line_len = new_width;
            area.contents = new_contents;
            area.draw(term)?;
            Ok(())
        }

        if tree {
            let contents = self.load_tree_contents(&mut 0, self.root.clone());
            refresh(self.term, &mut self.tree, contents)?;
            // let new_contents = self.load_tree_contents(&mut 0, self.root.clone());

            // let tree = &mut self.tree;
            // let new_width = new_contents.iter()
            //     .fold(0, |largest, line| max(largest, line.iter()
            //         .fold(0, |len, piece| len + piece.string.chars().count())));
            // let new_height = new_contents.len();

            // let farthest_right = tree.curr_pos.x + tree.size.x;
            // if farthest_right > new_width {
            //     tree.curr_pos.x -= farthest_right - new_width;
            // }

            // let farthest_down = tree.curr_pos.y + tree.size.y;
            // if farthest_down > new_height {
            //     tree.curr_pos.x -= farthest_down - new_height;
            // }

            // tree.longest_line_len = new_width;
            // tree.contents = new_contents;
            // tree.draw(self.term)?;
        }
        if contents {
            let contents = self.load_dir_contents();
            refresh(self.term, &mut self.tree, contents)?;
            // let contents = self.load_dir_contents();
            // self.dir_contents.longest_line_len = contents.iter()
            //     .fold(0, |largest, line| max(largest, line.iter()
            //         .fold(0, |len, piece| len + piece.string.chars().count())));
            // self.dir_contents.contents = contents;
            // self.dir_contents.draw(self.term)?;
        }
        // TODO scroll up/left if needed

        Ok(())
    }

    // returns a list of possible directories which match the searched name/path
    fn to_directory(&self, path: &str) -> Vec<DirectoryRef> {
        let parts: Vec<&str> = path.split("/").collect();
        
        // narrows down the possible valid directories part by part of the path specified
        let mut possible = vec![self.root.clone()];
        for part in &parts {
            let mut new: Vec<DirectoryRef> = vec![];
            for dir in &possible {
                self.to_directory_helper(part, dir.clone(), &mut new);
            }
            possible = new;
        }
        
        possible
    }
    
    // recursively goes through each directory in the tree and returns all matches
    fn to_directory_helper(&self, path: &str, curr_dir: DirectoryRef, possible: &mut Vec<DirectoryRef>) {
        if curr_dir.borrow().name == path {
            possible.push(curr_dir.clone());
        }
        if self.closed_dirs.contains(&curr_dir) {
            return;
        }

        let clone = curr_dir.clone();
        for dir in &clone.borrow().directories {
            self.to_directory_helper(path, dir.clone(), possible);
        }
    }

    
}

fn draw_outline(term: &Term, selected_panel: CurrentArea) -> io::Result<()> {
    let (height, width) = {
        let size = term.size();
        (size.0 as usize, size.1 as usize)
    };

    let line_x = (width as f64 * 0.5) as usize;

    term.move_cursor_to(0, 0)?;
    term.clear_line()?;
    term.write_line("DirMan")?;

    let print_with_color = |text: &str, colored_list: Vec<CurrentArea>| -> io::Result<()> {
        let colored = if colored_list.contains(&selected_panel) {
            format!("{}", text.color(Color::Red))
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

fn load_dir(dir_path: PathBuf, parent: Option<DirectoryRef>) -> io::Result<DirectoryRef> {
    // let mut files: Vec<FileRef> = vec![];
    // let mut directories: Vec<DirectoryRef> = vec![];

    let this_dir = Rc::new(RefCell::new(Directory::new(dir_path.clone(), vec![], vec![], parent)));

    for entry in fs::read_dir(&dir_path)? {
        let entry = entry?;
        let entry_path = entry.path();
        
        if entry.file_type()?.is_dir() {
            this_dir.borrow_mut().directories.push(load_dir(entry_path, Some(this_dir.clone()))?);
        } else {
            this_dir.borrow_mut().files.push(Rc::new(RefCell::new(File::new(entry.path()))));
        }
    }

    Ok(this_dir)
}

fn main() -> io::Result<()> {
    // parse command line arguments and extract directory
    let path = env::current_dir()?;
    if !path.is_dir() {
        println!("Input directory does not exist");
        return Ok(());
    }

    // construct directory tree
    let root = load_dir(path, None)?;

    let term = Term::stdout();

    // find dimensions for screen areas
    let size = Vector2 { x: term.size().1 as usize, y: term.size().0 as usize };
    if size.x < 75 || size.y < 10 {
        println!("Terminal window too small; terminal must be at least 75 units wide and 15 tall");
        return Ok(());
    }

    for _ in 0..=size.y {
        term.write_line("")?;
    }
    draw_outline(&term, CurrentArea::Command)?;

    let mut manager = StateManager::init(&term, root.clone())?;

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
                manager.term.move_cursor_to(3, manager.term.size().0 as usize - 1)?;
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
