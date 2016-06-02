extern crate gtk;
extern crate gdk;
extern crate quickersort;

mod systemd;
mod gui {
    pub mod gtk3;
}

fn main() {
    gui::gtk3::launch();
}
