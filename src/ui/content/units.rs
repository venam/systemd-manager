use gtk::*;
use pango::*;
use sourceview::*;
use systemd::{self, Kind};

pub struct Units {
    pub container: Box,
    pub selection: UnitsSelection,
    pub content:   UnitsContent,
}

impl Units {
    pub fn new() -> Units {
        let container = Box::new(Orientation::Horizontal, 0);
        let selection = UnitsSelection::new();
        let content = UnitsContent::new();

        container.pack_start(&selection.container, false, false, 0);
        container.pack_start(&content.container, true, true, 0);

        Units { container, selection, content }
    }
}

pub struct UnitsSelection {
    pub container:     Box,
    pub search:        SearchEntry,
    pub unit_switcher: StackSwitcher,
    pub units_stack:   Stack,
    pub system_units:  ListBox,
    pub user_units:    ListBox,
    pub refresh:       Button,
}

impl UnitsSelection {
    pub fn new() -> UnitsSelection {
        let container = Box::new(Orientation::Vertical, 3);
        container.set_property_width_request(300);

        let search = SearchEntry::new();

        let system_units = ListBox::new();
        let system_scroller = ScrolledWindow::new(None, None);
        system_scroller.add(&system_units);

        let user_units = ListBox::new();
        let user_scroller = ScrolledWindow::new(None, None);
        user_scroller.add(&user_units);

        let units_stack = Stack::new();
        let unit_switcher = StackSwitcher::new();
        unit_switcher.set_stack(&units_stack);
        units_stack.add_titled(&system_scroller, "System", "System");
        units_stack.add_titled(&user_scroller, "User", "User");

        let refresh = Button::new();
        refresh.set_image(&Image::new_from_icon_name("view-refresh-symbolic", 4));

        let switch_box = Box::new(Orientation::Horizontal, 0);
        switch_box.pack_start(&unit_switcher, false, false, 0);
        switch_box.pack_start(&refresh, false, false, 0);
        switch_box.set_halign(Align::Center);

        container.pack_start(&switch_box, false, false, 0);
        container.pack_start(&search, false, false, 0);
        container.pack_start(&units_stack, true, true, 0);
        container.set_border_width(3);

        UnitsSelection {
            container,
            units_stack,
            unit_switcher,
            system_units,
            user_units,
            search,
            refresh,
        }
    }

    pub fn update_list(&self, kind: Kind, new_items: &[systemd::Unit]) {
        let units = if kind == Kind::System { &self.system_units } else { &self.user_units };

        units.get_children().into_iter().for_each(|widget| widget.destroy());
        new_items.into_iter().for_each(|item| {
            let label = Label::new(item.name.as_str());
            label.set_halign(Align::Start);
            label.set_margin_left(5);
            label.set_margin_right(15);
            units.insert(&label, -1);
        });
    }
}

pub struct UnitsContent {
    pub container:   Box,
    pub description: Label,
    pub enabled:     Button,
    pub active:      Button,
    pub notebook:    UnitsNotebook,
}

impl UnitsContent {
    pub fn new() -> UnitsContent {
        let container = Box::new(Orientation::Vertical, 3);
        let info_bar = Box::new(Orientation::Horizontal, 3);
        let description = Label::new(None);

        description.set_markup("<b>Unit description here...</b>");
        description.set_halign(Align::Start);
        description.set_margin_left(3);

        let enabled = Button::new_with_label("Enable");
        let active = Button::new_with_label("Start");
        let mask = Button::new_with_label("Mask");
        let delete = Button::new_with_label("Delete");

        let extra_box = Box::new(Orientation::Vertical, 0);
        extra_box.pack_start(&mask, false, false, 0);
        extra_box.pack_start(&delete, false, false, 0);

        let extra = MenuButton::new();
        let menu = PopoverMenu::new();
        extra.set_popover(&menu);
        menu.add(&extra_box);
        extra_box.show_all();

        let notebook = UnitsNotebook::new();

        info_bar.pack_start(&description, true, true, 0);
        info_bar.pack_start(&enabled, false, false, 0);
        info_bar.pack_start(&active, false, false, 0);
        info_bar.pack_start(&extra, false, false, 0);

        container.pack_start(&info_bar, false, false, 0);
        container.pack_start(&notebook.container, true, true, 0);
        container.set_margin_right(3);
        container.set_margin_top(3);
        container.set_margin_bottom(3);

        UnitsContent { container, description, enabled, active, notebook }
    }
}

pub struct UnitsNotebook {
    pub container:         Notebook,
    pub file_buff:         Buffer,
    pub file_save:         Button,
    pub journal_buff:      TextBuffer,
    pub dependencies_buff: TextBuffer,
}

impl UnitsNotebook {
    pub fn new() -> UnitsNotebook {
        let file_box = Box::new(Orientation::Vertical, 0);
        let file_buff = Buffer::new(None);
        let file_view = View::new_with_buffer(&file_buff);
        let file_save = Button::new_with_label("Save");
        let scroller = ScrolledWindow::new(None, None);
        scroller.add(&file_view);
        file_box.pack_start(&scroller, true, true, 0);
        file_box.pack_start(&file_save, false, false, 0);

        let journal_buff = TextBuffer::new(None);
        let journal_view = TextView::new_with_buffer(&journal_buff);
        let journal_scroller = ScrolledWindow::new(None, None);
        journal_scroller.add(&journal_view);

        let dependencies_buff = TextBuffer::new(None);
        let dependencies_view = TextView::new_with_buffer(&dependencies_buff);
        let dependencies_scroller = ScrolledWindow::new(None, None);
        dependencies_scroller.add(&dependencies_view);

        configure_source_view(&file_view, &file_buff);
        journal_view.set_border_width(5);
        dependencies_view.set_border_width(5);

        let container = Notebook::new();
        container.set_show_tabs(true);
        container.add(&file_box);
        container.add(&journal_scroller);
        container.add(&dependencies_scroller);
        container.set_tab_label_text(&file_box, "File");
        container.set_tab_label_text(&journal_scroller, "Journal");
        container.set_tab_label_text(&dependencies_scroller, "Dependencies");
        expand_tab(&container, &file_box);
        expand_tab(&container, &journal_scroller);
        expand_tab(&container, &dependencies_scroller);

        UnitsNotebook { container, file_buff, file_save, journal_buff, dependencies_buff }
    }
}

fn expand_tab<W: IsA<Widget>>(container: &Notebook, child: &W) {
    container.get_tab_label(child).map(|tab| {
        let _ = tab.set_property("expand", &Value::from(&true));
        let _ = tab.set_property("fill", &Value::from(&true));
    });
}

fn configure_source_view(view: &View, buff: &Buffer) {
    WidgetExt::override_font(view, &FontDescription::from_string("monospace"));

    LanguageManager::new().get_language("ini").map(|ini| buff.set_language(&ini));

    view.set_show_line_numbers(true);
    view.set_monospace(true);
    view.set_insert_spaces_instead_of_tabs(true);
    view.set_indent_width(4);
    view.set_smart_backspace(true);
    view.set_right_margin(100);
    view.set_left_margin(10);
    view.set_show_right_margin(true);
    view.set_background_pattern(BackgroundPatternType::Grid);
}
