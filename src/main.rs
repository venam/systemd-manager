extern crate gtk;
extern crate gdk;
extern crate quickersort;

mod systemd_gui;     // Contains all of the heavy GUI-related work
mod systemd;

fn main() {
    systemd_gui::launch();
}
