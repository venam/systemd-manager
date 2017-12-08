extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate gtk;
extern crate pango;
extern crate sourceview;
extern crate systemd_manager;

mod ui;
pub use systemd_manager::systemd;

use ui::App;

fn main() { App::new().connect_events().then_execute() }
