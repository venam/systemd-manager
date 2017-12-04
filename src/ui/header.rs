use gtk::*;

pub struct Header {
    pub container: HeaderBar,
    pub views:     StackSwitcher,
    pub address:   Entry,
    pub menu:      MenuButton,
}

impl Header {
    pub fn new() -> Header {
        let container = HeaderBar::new();
        let address = Entry::new();
        let views = StackSwitcher::new();
        let menu = MenuButton::new();
        menu.set_image(&Image::new_from_icon_name("open-menu-symbolic", 4));

        container.pack_start(&views);
        container.set_show_close_button(true);
        container.set_title("Systemd Manager");
        container.set_subtitle("localhost");
        container.pack_end(&menu);

        Header { container, views, address, menu }
    }
}
