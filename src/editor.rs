use crate::terminal::{WindowSize,Position};
use crate::buffer::{Buffer,Direction};
use crossterm::{QueueableCommand,cursor, ExecutableCommand};
use std::{io::{Write,stdout,stdin}, cmp};
use crossterm::event::{read,Event};
use crossterm::event::{KeyEvent,KeyCode,KeyModifiers};
use crossterm::terminal::ClearType;
use crossterm::execute;
use crossterm::style::{Color,Colors,Print,SetColors};
use colored::*;
enum EditorMode{
    Edit,
    Replace,
    Mark,
}


pub struct Editor{
    should_close: bool,
    window_size: WindowSize,
    cursor_pos: Position,
    buffer: Buffer,
    mode: EditorMode,
    mark_delta: (Position,Position),
    status_message: String,
    draw_accumulator: u32,
    write_status: bool,
    pub line_numbers: bool,
    offset: Position,

}

impl Default for Editor{
    fn default()->Self{
        Self{
            should_close: false,
            window_size: (0,0).into(),
            cursor_pos: (0,0).into(),
            buffer: Default::default(),
            mode: EditorMode::Edit,
            mark_delta: (Default::default(),Default::default()),
            status_message: String::new(),
            draw_accumulator: 0,
            write_status: true, // Initialized to true because the "scratch" buffer will be "written" to since there is no data
            line_numbers: false,
            offset: (0,0).into(),
        }
    }
}

impl Editor{
    fn init(&mut self){
        execute!(stdout(),
                crossterm::terminal::EnterAlternateScreen
        ).expect("red: error: failed to enter alternate screen");

        crossterm::terminal::enable_raw_mode().expect("red: error: failed to enable raw mode");

        let mut stdout = stdout();
        stdout.execute(cursor::MoveTo(self.cursor_pos.r as u16,self.cursor_pos.c as u16)).expect(format!("red: error: failed to move cursor to {},{}",self.cursor_pos.r,self.cursor_pos.c).as_str());
        self.window_size = crossterm::terminal::size().unwrap().into();
    }
    pub(crate) fn run(&mut self)-> Result<(),std::io::Error>{
        self.init();
        loop{
            if self.should_close{
                if self.write_status == false{
                    match self.prompt(format!("Open buffer {} contains data, write to disk (yes/no/cancel)? ",self.buffer.name).as_str()).trim().to_lowercase().as_str(){
                        "n" | "no"  => break,
                        "y" | "yes" => {
                            self.write_to_disk();
                            break
                        },
                        "c" | "cancel" =>{
                            self.should_close = false;
                        }
                        _ =>{},

                    }
                }
                else{
                    break;
                }
                self.update_status("");
            }
            if crossterm::event::poll(std::time::Duration::from_millis(500)).unwrap(){
                match read()?{
                    Event::Key(k) =>{
                        self.process_keypress(k);
                    },
                    Event::Resize(width,height) =>{
                        self.window_size = (width,height).into();
                    }
                    _=>{},
                }
            }
            self.draw_lines();
            self.draw_status();
            self.draw_accumulator += 1;
        }
        Ok(())
    }

    fn write_to_disk(&mut self){
        let file_name: String = match self.buffer.name.as_str(){
            "scratch" =>{
                 self.prompt(&"File name to write:  ".blue())
            },
            _ => {self.buffer.name.clone()},
        };
        let file_name = file_name.trim().to_string();
        let message = self.buffer.write(Some(file_name.clone()));
        self.update_status(message.unwrap_or("Error failed to write to disk!".into()).as_str());
        self.write_status = true;
    }

    fn prompt_write(&mut self){
        match self.prompt(format!("Open buffer {} contains data, write to disk (yes/no)? ",self.buffer.name).as_str()).trim().to_lowercase().as_str(){
            "y" | "yes" => self.write_to_disk(),
            "n" | "no"=> {
                return
            },
            _ =>{},

        }
    }

    fn new_buffer(&mut self){
        if self.write_status == false{
            self.prompt_write();
        }
        // assuming that if the user answers no, all data is discarded.
        self.buffer = Default::default();
        self.cursor_pos = (0,0).into();
        self.update_status("Created a new scratch buffer.");
        self.write_status = true;
    }

    fn prompt(&mut self, message: &str) -> String{
        self.update_status(message);
        execute!(
                stdout(),
                cursor::MoveTo(0 as u16,self.window_size.rows),
                crossterm::terminal::Clear(ClearType::CurrentLine),
                Print(self.status_message.clone()),
                cursor::MoveTo(message.len() as u16,self.window_size.rows),
                ).ok();

        let mut buf = String::new();
        crossterm::terminal::disable_raw_mode().ok();
        stdin().read_line(&mut buf).expect("red: error: failed to read in from stdin");
        crossterm::terminal::enable_raw_mode().ok();
        buf
    }

    pub fn open_file(&mut self, file_name: &str){
        let file_name = file_name.replace("\"","");
        if self.write_status ==false{
            self.prompt_write();
        }
        let new_buffer = Buffer::open(file_name.trim().as_ref());
        if !new_buffer.1{
            self.buffer = new_buffer.0;
            self.cursor_pos = Default::default();
            if !self.buffer.read_only{
                self.update_status(format!("Successfully opened file {}",file_name).as_str());
            }
        }
        else{
            match self.prompt(format!("Failed to open file {}. Create a file with the same name? ",file_name).as_str()).trim().to_lowercase().as_str(){
                "yes" | "y" =>{
                    let file_name = file_name;
                    self.buffer = new_buffer.0;
                    self.buffer.write(None);
                },
                "n" | "no" =>{
                    self.update_status("");
                },
                _ =>{},
            }
        }
        stdout().execute(crossterm::terminal::Clear(ClearType::All)).ok();
    }


    fn update_status(&mut self, message: &str){
        self.status_message = message.into();
        self.draw_accumulator = 0;
    }

    fn process_keypress(&mut self, key_event: KeyEvent){
        match (key_event.modifiers,key_event.code){
            (KeyModifiers::CONTROL,KeyCode::Char('q'))=>{
                self.should_close = true;
            },
            (KeyModifiers::CONTROL,KeyCode::Char('w'))=> {
                self.write_to_disk();
            },
            (KeyModifiers::CONTROL,KeyCode::Char('o'))=> {
                let mut file_name = self.prompt("File to be opened: ");
                file_name = file_name.trim().to_string();
                self.open_file(file_name.as_str());
            },
            (KeyModifiers::CONTROL,KeyCode::Char('n'))=> {
                self.new_buffer();
            },
            (KeyModifiers::CONTROL,KeyCode::Char('j'))=> {
                self.prompt_jump();
            },
            (KeyModifiers::CONTROL,KeyCode::Char('f'))=>{
                let search_text = self.prompt("Find: ");
                self.search(&search_text);
            }
            (_,KeyCode::F(num)) =>{
                match num {
                    2=>{
                        let path = std::env::current_exe().unwrap().into_os_string().into_string().unwrap();
                        self.open_file((path +"/LICENSE").as_str());
                    },
                    4=>{
                        self.line_numbers = !self.line_numbers;
                    },
                    _ => {},
                }
            },

            (_,KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right| KeyCode::Home | KeyCode::End) =>{
                self.move_cursor(key_event.code);
            },
            (_,KeyCode::Delete) =>{
                self.buffer.remove(self.cursor_pos,Direction::Forward,1);
            },
            (_,KeyCode::Backspace) =>{
                let len = self.buffer.len();
                self.buffer.remove(self.cursor_pos,Direction::Backward,1);
                if len  != self.buffer.len(){
                    self.move_cursor(KeyCode::Up);
                }
                else {
                    self.move_cursor(KeyCode::Left);
                }
            },
            (_,KeyCode::Enter)=>{
                if !self.buffer.read_only{
                    self.write_status = false;
                }
                self.buffer.insert(self.cursor_pos,'\n');
                self.move_cursor(KeyCode::Down);
            },
            (_,KeyCode::Char(c))=>{
                if !self.buffer.read_only{
                    self.write_status = false;
                }                    self.buffer.insert(self.cursor_pos,c);
                self.move_cursor(KeyCode::Right);
            },
            (_,KeyCode::Tab)=>{
                if !self.buffer.read_only{
                    self.write_status = false;
                }
                self.buffer.insert(self.cursor_pos,'\t');
                self.move_cursor(KeyCode::End);
            }
            _=>{},
        }
    }

    fn draw_lines(&mut self){
        let mut stdout = stdout();

        execute!(stdout,
                cursor::MoveTo(0,0),
                cursor::Hide,
                crossterm::terminal::Clear(ClearType::FromCursorDown),
        ).ok();

        let offset = (self.cursor_pos.r/(self.window_size.rows-1))*(self.window_size.rows-1);

        for i in 0..self.window_size.rows as u16{
            if ((offset + i) as usize) < self.buffer.len(){
                let line = self.buffer.get((offset + i) as usize).unwrap();
                if self.line_numbers {
                    self.offset.c = ((((self.buffer.len() - 1) as f32).log10()) as u16) + 2;
                    let size = self.offset.c as usize - 1;
                    execute!(
                        stdout,
                        cursor::MoveTo(0,i),
                        SetColors(Colors::new(Color::DarkYellow,Color::Black)),

                        Print(match self.line_numbers{
                            true => format!("{:>size$}",i + offset),
                            false => String::default(),
                        }),
                        SetColors(Colors::new(Color::Reset,Color::Reset)),
                        cursor::MoveTo(self.offset.c,i + self.offset.r),
                    ).unwrap();
                }
                else{
                    self.offset.c = 0;
                    execute!(
                        stdout,
                        cursor::MoveTo(0,i + self.offset.r),
                    ).unwrap();
                }
                if line.len() > self.window_size.cols as usize{
                    // write!(stdout,"{}",line.as_str()[0..self.window_size.cols as usize]).ok();
                }
                else{

                    write!(stdout,"{}",line).ok();
                }
            }

        }

        self.draw_modeline();
        stdout.queue(cursor::MoveTo(self.cursor_pos.c + self.offset.c,self.cursor_pos.r%(self.window_size.rows-1))).ok();
        stdout.queue(cursor::Show).ok();
        stdout.flush().ok();
    }

    fn draw_status(&mut self){

        execute!(stdout(),
        cursor::MoveTo(0 as u16,self.window_size.rows),
        crossterm::terminal::Clear(ClearType::CurrentLine),
        Print(self.status_message.trim()),
        cursor::MoveTo(self.cursor_pos.c + self.offset.c,self.cursor_pos.r%(self.window_size.rows-1)),
        ).ok();

        if self.draw_accumulator == 20{
            self.status_message.truncate(0);
            self.draw_accumulator = 0;
        }
    }

    fn draw_modeline(&self){
        let len = self.window_size.cols as usize;
        let file_status_str = match self.write_status{
            true =>{
                if self.buffer.read_only{
                    "readonly"
                }
                else{
                    ""
                }
            },
            false =>{
                "modified"
            },
        };

        let bpos= if file_status_str.len() > 0{
            len - (file_status_str.len()*4) +1
        }
        else {
            0
        };
        let modeline = format!("{:<20}{:>5}:{:<5}{:>bpos$}",self.buffer.name,
                                                    self.cursor_pos.r,
                                                    self.cursor_pos.c,
                                                    file_status_str);
        let modeline = format!("{:len$}",modeline);
        execute!(
                stdout(),
        cursor::MoveTo(0,self.window_size.rows-2),
        SetColors(Colors::new(Color::Black,Color::White)),
        Print(modeline),
        SetColors(Colors::new(Color::Reset,Color::Reset)),
        ).unwrap();

    }


    fn move_cursor(&mut self, code: KeyCode) {


        match code{
            KeyCode::Up =>{
                if self.cursor_pos.r != 0{
                    self.cursor_pos.r = cmp::max(self.cursor_pos.r-1,0);
                }
                match self.buffer.get(self.cursor_pos.r.into()){
                    Some(line)=>{
                        self.cursor_pos.c = cmp::max(0,line.len() as u16)
                    }
                    None => self.cursor_pos.c =0
                }
            },
            KeyCode::Down =>{
                if self.buffer.len() == 0{
                    return
                }
                self.cursor_pos.r = cmp::min(self.cursor_pos.r+1,self.buffer.len() as u16 -1);
                match self.buffer.get(self.cursor_pos.r.into()){
                    Some(line)=>{
                        self.cursor_pos.c = cmp::max(0,line.len() as u16)
                    }
                    None => self.cursor_pos.c =0
                }
            },
            KeyCode::Left =>{
                if self.cursor_pos.c != 0{
                    self.cursor_pos.c = cmp::max(self.cursor_pos.c-1,0);
                }
            },
            KeyCode::Right =>{
                let line =self.buffer.get(self.cursor_pos.r as usize);
                if line == None{
                    return
                };
                let line = line.unwrap();
                self.cursor_pos.c = cmp::min(self.cursor_pos.c+1, line.len()as u16);
            },
            KeyCode::Home =>{
                let line =self.buffer.get(self.cursor_pos.r as usize);
                if line == None{
                    return
                };
                self.cursor_pos.c = 0;
            },
            KeyCode::End =>{
                let line =self.buffer.get(self.cursor_pos.r as usize);
                if line == None{
                    return
                };
                self.cursor_pos.c = line.unwrap().len() as u16;
            },
            _=>{},
        }
    }

    fn search(&mut self, s: &String){
        let line = self.buffer.find(s);
        if line.is_ok(){
            self.status_message = "found string ".to_string();
            let line = line.unwrap();
            let substr = self.buffer.get(line).unwrap();
            if let Some(index) = substr.find(s) {
                self.cursor_pos.r = line as u16;
                self.cursor_pos.c = index as u16;
            }
        }
    }


    fn prompt_jump(&mut self) {
        let result = self.prompt("Line to jump to: ");
        let result = result.trim();
        let mut res_i  = result.parse::<u16>().unwrap_or(self.cursor_pos.r);
        if res_i > (self.buffer.len()-1) as u16{
            res_i = self.cursor_pos.r
        }
        self.cursor_pos.r = res_i;
    }
}

pub fn cleanup(){
    execute!(stdout(),
    crossterm::terminal::LeaveAlternateScreen
    ).expect("red: error: failed to enter alternate screen");
}
