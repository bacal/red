use std::fmt::Error;
use std::fs::File as RFile;
use std::io::{BufReader, BufWriter, ErrorKind};
use std::path::{Path,PathBuf};
use std::io::prelude::*;
use crate::terminal::Position;

pub(crate) struct Buffer{
    pub name: String,
    lines: Vec<String>,
    path: PathBuf,
    pub read_only: bool,
}

pub(crate) enum Direction{
    Forward,
    Backward,
}

//impl Index<usize> for Buffer{
//    type Output = String;
//    fn index(&self, )
//}

impl Default for Buffer{
    fn default()->Self{
        Self{
             lines: vec![],
             name: String::from("scratch"),
             read_only: false,
             path: PathBuf::new(),
        }
    }
}

impl Buffer{
    pub fn open(file_path: &str)->(Self,bool){
        let path = PathBuf::from(file_path);
        let mut name = path.file_name().unwrap_or_default().to_str().unwrap().to_string();
        let mut read_only = false;
        let file = RFile::open(path.clone());
        let mut new = true;
        let vec: Vec<String> = match file {
            Ok(f) =>{
                let reader = BufReader::new(f.try_clone().unwrap());
                let lines: Vec<String> = reader.lines().map(|l| l.expect("red: error: could not parse line")).collect();
                let path = path.as_path();
                name = path.file_name().unwrap().to_os_string().into_string().unwrap();
                drop(f);
                std::env::set_current_dir(&path.parent().unwrap_or(Path::new("."))).ok();
                new = false;

                lines
            },
            Err(e) =>{
                vec![]
            }
        };

        if let Ok(f) = RFile::open(path.clone()){
            let stats = f.metadata().unwrap();
            read_only = stats.permissions().readonly();
        }


        (Self{
            lines: vec,
            name: name,
            read_only: read_only,
            path
        },new)

    }


    pub fn write(&mut self, file_name: Option<String>) -> Result<String,std::io::Error>{
        let create = file_name.is_some();
        let file_name = file_name.unwrap_or(self.name.to_string());
        let fully_qualified_file_path =if create{
            Path::new(file_name.as_str())
        }
        else{
            self.path.as_path()
        };
        let mut outfile = match RFile::create(fully_qualified_file_path){
            Err(e) => {
                return Err(e);
            }
            Ok(file) =>{
                file
            }
        };

        let _ : Vec<_> = self.lines.iter().map(|l| outfile.write((l.clone() + "\n").as_bytes())).collect();
        self.name = String::from(fully_qualified_file_path.file_name().unwrap().to_str().unwrap());
        Ok(format!("Wrote {} lines to disk.",self.lines.len()))
    }

    pub(crate) fn find(&self, s: &String) -> Result<usize, ()>{
        for (p,l) in self.lines.iter().enumerate(){
            if l.contains(s){
                return Ok(p);
            }
        }
        Err(())
    }

    pub(crate) fn insert(&mut self, pos: Position, c: char){
        if self.read_only{
            return
        }
        if c=='\n'{
            self.insert_newline(pos);
        }
        else if c== '\t'{
            self.insert_tab(pos);
        }
        else if self.lines.len() != 0{
            let row = &mut self.lines[pos.r as usize];
            row.insert(pos.c as usize, c);
        }
        else{
            self.insert_newline((0,0).into());
            let line = &mut self.lines[0];
            line.push(c);
        }
    }


    fn insert_tab(&mut self, pos: Position) {
        let tab = "    ";
        if self.lines.len() == 0{
            self.lines.push(Default::default());
            self.lines[0].insert_str(0, tab);
        }
        else{
            let row = &mut self.lines[pos.r as usize];
            row.insert_str(pos.c as usize, tab);
        }
    }

    fn insert_newline(&mut self, pos: Position){

        if self.lines.len() == 0{
            self.lines.push(Default::default());
        }
        else if pos.r as usize == self.lines.len()-1{
            self.lines.push(Default::default());
        }
        else{
            self.lines.insert(pos.r as usize +1,Default::default());
        }
        if pos.c as usize != self.lines[pos.r as usize].len(){
            let view = self.lines[pos.r as usize].clone();
            let view = &view.as_str()[pos.c as usize..];
            self.lines[pos.r as usize +1].insert_str(0,view);
            let newsize = self.lines[pos.r as usize].len() - view.len();
            self.lines[pos.r as usize].truncate(newsize);
        }
    }

    pub(crate) fn len(&self) -> usize{
        self.lines.len()
    }

    pub(crate) fn get(&self, index: usize)->Option<&String>{
        match self.lines.get(index){
            Some(line) =>{Some(&line)},
            None => None
        }
    }

    pub(crate) fn replace(&mut self, pos: Position){
        //write replace mode functionality here.
        todo!();
    }

    pub(crate) fn remove(&mut self, pos: Position, direction: Direction, _num_chars: i32){
        if self.lines.len()==0 || self.lines.len() == pos.r as usize{
            return;
        }
        let line = &mut self.lines[pos.r as usize];
        match direction{
            Direction::Forward =>{
                if line.len() == 0{
                    self.lines.remove(pos.r as usize);
                    return;
                }
                else{
                    if pos.c != line.len() as u16{
                        line.remove(pos.c as usize);
                    }
                    else if pos.r as usize +1 != self.lines.len(){
                            self.remove_and_concat(pos);
                    }
                }

            },
            Direction::Backward =>{
                if line.len() == 0{
                    if self.lines.len() == 1{
                        self.lines.pop();
                    }
                    else if self.lines.len() ==0{
                        return
                    }
                    else{
                        self.lines.remove(pos.r as usize);
                    }
                    return;
                }
                else if pos.c == line.len() as u16{
                    line.pop();
                }
                else{
                    if pos.c != 0{
                        line.remove(pos.c as usize);
                    }
                }
            },
        }
    }

    fn remove_and_concat(&mut self, pos: Position){
        let next_row = self.lines[pos.r as usize+1].clone();
        let row = &mut self.lines[pos.r as usize];
        row.push_str(next_row.as_str());
        self.lines.remove(pos.r as usize + 1);
    }

}