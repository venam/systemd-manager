extern crate dbus;
extern crate gtk;
extern crate pango;
extern crate quickersort;
extern crate sourceview;
extern crate failure;
#[macro_use]
extern crate failure_derive;

mod ui;
pub mod systemd;

use ui::App;

fn main() { App::new().connect_events().then_execute() }
