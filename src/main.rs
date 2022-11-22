mod terminal;
mod buffer;
mod editor;
use editor::Editor;
//use clap::{arg,Command};
//
//fn cli() -> Command{
//    Command::new("red")
//    .about("A barebones text edtitor")
//    .subcommand_required(false)
//    .allow_external_subcommands(false)
//    .arg_required_else_help(false)
//    .arg(arg!([FILE] "Fully qualified file name").value_parser(clap::value_parser!(String)))
//}

fn main() {

    let mut e = Editor::default();
    e.run();
    editor::cleanup();
    crossterm::terminal::disable_raw_mode().expect("red: error: failed to disable raw mode!");

}
