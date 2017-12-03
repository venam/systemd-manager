use gtk::*;
pub const VIEWS: &[&str] = &["Units", "Boot", "Create"];

pub struct Header {
    pub container: HeaderBar,
    pub view:      Views,
    pub address:   Entry,
}

impl Header {
    pub fn new() -> Header {
        let container = HeaderBar::new();
        let address = Entry::new();
        let view = Views::new();

        container.pack_start(&view.button);
        container.set_show_close_button(true);
        container.set_title("Systemd Manager");
        container.set_subtitle("localhost");

        Header { container, view, address }
    }
}

pub struct Views {
    pub button: MenuButton,
    pub units:  Button,
    pub boot:   Button,
    pub create: Button,
    pub list:   Box,
}

fn view_button(id: usize) -> Button {
    let button = Button::new_with_label(VIEWS[id]);
    button.set_relief(ReliefStyle::None);
    button
}

impl Views {
    pub fn new() -> Views {
        let units = view_button(0);
        let boot = view_button(1);
        let create = view_button(2);

        let view_box = Box::new(Orientation::Vertical, 0);
        view_box.pack_start(&units, false, false, 0);
        view_box.pack_start(&boot, false, false, 0);
        view_box.pack_start(&create, false, false, 0);
        view_box.set_border_width(5);
        view_box.show_all();

        let view_pop = PopoverMenu::new();
        view_pop.add(&view_box);

        let view = MenuButton::new();
        view.set_popover(&view_pop);
        view.add(&Label::new("Units"));

        Views { button: view, list: view_box, units, boot, create }
    }
}
