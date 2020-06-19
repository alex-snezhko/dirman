use std::env;
use std::fs;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use console::Term;

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
    screen_offset: (usize, usize),
    size: (usize, usize),
    used: (usize, usize),
    curr_pos: (usize, usize),
}

impl ScrollableArea {
    fn draw(&self, term: &Term) -> std::io::Result<()> {
        term.move_cursor_to(self.screen_offset.1 as usize, self.screen_offset.0 as usize)?;

        Ok(())
    }
}

struct TerminalManager {
    tree_area: ScrollableArea,
    contents_area: ScrollableArea,
    command_area: ScrollableArea,
}

fn main() -> std::io::Result<()> {
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
    let size = term.size();
    let (height, width) = (size.0 as usize, size.1 as usize);
    let line_x = (width as f64 * 0.6) as usize;

    let tree_area = ScrollableArea {
        screen_offset: (3, 1),
        size: (height - 6, line_x - 2),
        used: (0, 0),
        curr_pos: (0, 0),
    };

    // let contents_area = ScrollableArea {
    //     screen_offset: (3, line_x + 2),
    //     size: (height - 6, (width - line_x) - 2),
    //     used: (0, 0),
    //     curr_pos: (0, 0),
    // };

    // let command_area = ScrollableArea {
    //     screen_offset: (height - 2, 3),
    //     size: (1, width - 2),
    //     used: (0, 0),
    //     curr_pos: (0, 0),
    // };
    
    // let manager = TerminalManager {
    //     tree_area,
    //     contents_area,
    //     command_area,
    // };

    //draw(&term, &manager);
    draw_outline(height, width);
    draw_tree(&term, &tree_area, &root, &mut (tree_area.screen_offset.0, tree_area.screen_offset.1))?;

    

    //let abs_path = std::fs::canonicalize(&path)
    //    .expect(&format!("directory {} cannot be found", dir_str));

    Ok(())
}

fn load_dirs(curr_dir: &mut Directory, dir_path: &Path) -> std::io::Result<()> {
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

fn draw_tree(term: &Term, tree_area: &ScrollableArea, curr_dir: &Directory, curr_dir_pos: &mut (usize, usize)) -> std::io::Result<()> {
    let begin_pos = (curr_dir_pos.0, curr_dir_pos.1);

    let name = &curr_dir.name;
    term.move_cursor_to(begin_pos.1, begin_pos.0)?;
    term.write_str(name.to_str().unwrap())?;

    for (i, dir) in curr_dir.directories.iter().enumerate() {
        curr_dir_pos.1 = begin_pos.1 + 3;
        curr_dir_pos.0 += 1;

        term.move_cursor_to(begin_pos.1, curr_dir_pos.0)?;

        if i == curr_dir.directories.len() - 1 {
            term.write_str("└─")?; // U+2514 U+2500
        } else {
            term.write_str("├─")?; // U+251C U+2500
        }
        
        let init_y = begin_pos.0;
        draw_tree(term, tree_area, dir, curr_dir_pos)?;

        for i in init_y+1..curr_dir_pos.0 {
            term.move_cursor_to(begin_pos.1, i)?;
            term.write_str("│")?; // U+2502
        }
    }

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
    
}
