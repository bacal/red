use crate::terminal::{WindowSize,Position};
use crate::buffer::{Buffer,Direction};
use crossterm::{QueueableCommand,cursor, ExecutableCommand};
use std::{io::{Write,stdout,stdin}, cmp};
use crossterm::event::{read,Event};
use crossterm::event::{KeyEvent,KeyCode,KeyModifiers};
use crossterm::terminal::ClearType;
use crossterm::execute;
use crossterm::style::{Color,Colors,Print,SetColors};

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
    line_numbers: bool,
    line_offset: u16,
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
            line_offset: 0,
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
                        self.update_status(format!("window sized by {}x{}",width,height).as_str());
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
                self.prompt("File name to write:  ")
            },
            _ => {self.buffer.name.clone()},
        };
        let file_name = file_name.trim().to_string();
        let message = self.buffer.write(Some(file_name.clone()));
        self.update_status(message.as_str());
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

    fn open_file(&mut self, file_name: &str){
        if self.write_status ==false{
            self.prompt_write();
        }
        let new_buffer = Buffer::open(file_name.trim().as_ref());
        if new_buffer.name.as_str() != "scratch"{
            self.buffer = new_buffer;
            self.cursor_pos = Default::default();
            if !self.buffer.read_only{
                self.update_status(format!("Successfully opened file {}",file_name).as_str());
            }
        }
        else{
            match self.prompt(format!("Failed to open file {}. Create a file with the same name? ",file_name).as_str()).trim().to_lowercase().as_str(){
                "yes" | "y" =>{
                    let file_name = file_name;
                    self.buffer.name = file_name.to_string();
                    self.write_to_disk();
                    self.buffer = Buffer::open(file_name);
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

    // refactor this to use a match statement?
    fn process_keypress(&mut self, key_event: KeyEvent) {
        if key_event.code == KeyCode::Char('q') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_close = true;
        } else if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('o') {
            let mut file_name = self.prompt("File to be opened: ");
            file_name = file_name.trim().to_string();
            self.open_file(file_name.as_str());
        } else if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('w'){
            self.write_to_disk();
        } else if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('n'){
            self.new_buffer();
        }else if key_event.code == KeyCode::F(4){
            self.line_numbers = !self.line_numbers;
            self.line_offset = match self.line_numbers{
                true =>{
                    (self.cursor_pos.r.to_string().bytes().map(|_b| 1).sum::<u16>()) +1
                },
                false=>0,
            };
        }else if key_event.code == KeyCode::F(2){
            self.open_file("LICENSE");

        }

        else{
            stdout().execute(crossterm::terminal::Clear(ClearType::All)).ok();
            match key_event.code {
                KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                    self.move_cursor(key_event.code);
                },
                KeyCode::Delete =>{
                    self.buffer.remove(self.cursor_pos,Direction::Forward,1);
                }
                KeyCode::Backspace =>{
                    let len = self.buffer.len();
                    self.buffer.remove(self.cursor_pos,Direction::Backward,1);
                    if len  != self.buffer.len(){
                        self.move_cursor(KeyCode::Up);
                    }
                    else {
                        self.move_cursor(KeyCode::Left);
                    }
                }
                KeyCode::Home | KeyCode::End =>{self.move_cursor(key_event.code)}
                KeyCode::Enter=>{
                    if !self.buffer.read_only{
                        self.write_status = false;
                    }
                    self.buffer.insert(self.cursor_pos,'\n');
                    self.move_cursor(KeyCode::Down);
                }
                KeyCode::Char(c) => {
                    if !self.buffer.read_only{
                        self.write_status = false;
                    }                    self.buffer.insert(self.cursor_pos,c);
                    self.move_cursor(KeyCode::Right);
                },
                KeyCode::Tab => {
                    if !self.buffer.read_only{
                        self.write_status = false;
                    }
                    self.buffer.insert(self.cursor_pos, '\t');
                    self.move_cursor(KeyCode::Right);
                    self.move_cursor(KeyCode::Right);
                    self.move_cursor(KeyCode::Right);
                    self.move_cursor(KeyCode::Right);
                }
                _ => {},
            }
        }
    }

    fn draw_lines(&self){
        let mut stdout = stdout();
        stdout.execute(cursor::MoveTo(0,0)).ok();
        stdout.execute(cursor::Hide).ok();
        stdout.execute(crossterm::terminal::Clear(ClearType::All)).ok();
            for i in 0..self.window_size.rows as u16{
                if self.line_numbers{
                    execute!(
                            stdout,
                    cursor::MoveTo(0,i),
                    SetColors(Colors::new(Color::Black,Color::White)),
                    Print(format!("{:<2}",i)),
                    SetColors(Colors::new(Color::Reset,Color::Reset)),
                    ).unwrap();
                    stdout.execute(cursor::MoveTo(self.line_offset,i)).ok();
                }
                else{
                    stdout.execute(cursor::MoveTo(0,i)).ok();
                }

                if (i as usize ) < self.buffer.len(){
                    write!(stdout,"{}",self.buffer.get(i as usize).unwrap()).ok();
                }

        }

        self.draw_modeline();
        stdout.queue(cursor::MoveTo(self.cursor_pos.c + self.line_offset,self.cursor_pos.r)).ok();
        stdout.queue(cursor::Show).ok();
        stdout.flush().ok();
    }

    fn draw_status(&mut self){

        execute!(stdout(),
                 cursor::MoveTo(0 as u16,self.window_size.rows),
                 crossterm::terminal::Clear(ClearType::CurrentLine),
                 Print(self.status_message.trim()),
                 cursor::MoveTo(self.cursor_pos.c + self.line_offset,self.cursor_pos.r),
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
        let bpos = len - file_status_str.len()*4;
        let modeline = format!("{:^20}{}:{}{:>bpos$}",self.buffer.name,
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


}

pub fn cleanup(){
    execute!(stdout(),
    crossterm::terminal::LeaveAlternateScreen
    ).expect("red: error: failed to enter alternate screen");
}
