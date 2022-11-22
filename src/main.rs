mod terminal;
mod buffer;
mod editor;
use editor::Editor;
fn main() {
    let mut e = Editor::default();
    e.run();
    editor::cleanup();
    crossterm::terminal::disable_raw_mode().expect("red: error: failed to disable raw mode!");

}
