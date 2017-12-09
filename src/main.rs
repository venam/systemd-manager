extern crate failure;
extern crate gtk;
extern crate pango;
extern crate sourceview;
extern crate systemd_manager;

mod ui;

use ui::App;

fn main() { App::new().connect_events().then_execute() }
