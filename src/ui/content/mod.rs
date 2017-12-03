pub mod units;

pub use self::units::*;
use gtk::*;

pub struct Content {
    pub container: Stack,
    pub units:     Units,
}

impl Content {
    pub fn new(views: &StackSwitcher) -> Content {
        let container = Stack::new();
        let units = Units::new();
        container.add_titled(&units.container, "Units", "Units");
        views.set_stack(&container);
        Content { container, units }
    }
}
