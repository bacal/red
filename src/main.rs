mod terminal;
mod buffer;
mod editor;
use editor::Editor;

fn main() {
    // let args = Args::parse();
    let mut e = Editor::default();
    // println!("{}",args);
    // e.line_numbers = args.line_numbers;
    e.run();
    editor::cleanup();
    crossterm::terminal::disable_raw_mode().expect("red: error: failed to disable raw mode!");

}
