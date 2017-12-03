pub mod units;

pub use self::units::*;
use gtk::*;

pub struct Content {
    pub container: Stack,
    pub units:     Units,
}

impl Content {
    pub fn new() -> Content {
        let container = Stack::new();
        let units = Units::new();
        container.add_named(&units.container, "units");
        Content { container, units }
    }
}
