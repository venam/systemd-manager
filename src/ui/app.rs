use super::{Content, Header};
use super::dialogs::Dialogs;
use gtk;
use gtk::*;
use std::ops::DerefMut;
use std::process;
use std::sync::{Arc, RwLock};
use systemd::{self, Kind, Location, UnitStatus, Units};

const DESTRUCTIVE: &str = "destructive-action";
const SUGGESTED: &str = "suggested-action";

const CUSTOM_CSS: &str = r#"
.odd_row, .even_row {
    padding: 5px;
}

.even_row text {
    border-color: #09F;
}

grid textview text {
    background: transparent;
    border-bottom-width: 0.1em;
    border-style: solid;
}

button {
    padding:3px;
}

buttonbox > button {
    border-radius: 0;
}

row {
    border-bottom-width: 0.1em;
    border-style: solid;
    border-color: #AAA;
    padding: 0.25em;
}

row:selected, row:hover {
    background: @bg-color;
    border-left-width:.25em;
    border-style: solid;
    border-color: #AAA;
}

row:hover, row:selected {
    border-color: #09F;
}

row:hover label, row:selected label {
    color: #09F;
}

row:selected {
    font-weight: bold;
}

buttonbox > button:first-child {
    border-top-left-radius: 10px;
    border-bottom-left-radius: 10px;
}

buttonbox > button:last-child {
    border-top-right-radius: 10px;
    border-bottom-right-radius: 10px;
}"#;

pub struct App {
    pub window:  Window,
    pub header:  Header,
    pub content: Content,
}

/// A wrapped `App` which provides the capability to execute the program.
pub struct ConnectedApp(App);

impl ConnectedApp {
    /// Display the window, and execute the gtk main event loop.
    pub fn then_execute(self) {
        self.0.window.show_all();
        gtk::main();
    }
}

impl App {
    pub fn new() -> App {
        // Initialize GTK before proceeding.
        if gtk::init().is_err() {
            eprintln!("failed to initialize GTK Application");
            process::exit(1);
        }

        // Create a new top level window.
        let window = Window::new(WindowType::Toplevel);

        // Add a custom CSS style
        let screen = window.get_screen().unwrap();
        let grid_style = CssProvider::new();
        let _ = CssProviderExt::load_from_data(&grid_style, CUSTOM_CSS.as_bytes());
        StyleContext::add_provider_for_screen(&screen, &grid_style, STYLE_PROVIDER_PRIORITY_USER);

        // Create a the headerbar and it's associated content.
        let header = Header::new();
        // Create the content container and all of it's widgets.
        let content = Content::new(&header.views);

        // Set the headerbar as the title bar widget.
        window.set_titlebar(&header.container);
        // Set the title of the window.
        window.set_title("Systemd Manager");
        // Set the window manager class.
        window.set_wmclass("systemd-manager", "Systemd Manager");
        // The icon the app will display.
        window.set_default_size(800, 600);
        Window::set_default_icon_name("iconname");
        // Add the content to the window.
        window.add(&content.container);


        // Programs what to do when the exit button is used.
        window.connect_delete_event(move |_, _| {
            main_quit();
            Inhibit(false)
        });

        // Return the application structure.
        App { window, header, content }
    }

    /// Creates external state, and maps all of the UI functionality to the UI.
    pub fn connect_events(self) -> ConnectedApp {
        let system_units = Units::new(Kind::System, Location::Localhost).unwrap();
        let user_units = Units::new(Kind::User, Location::Localhost).unwrap();

        update_list(&self.content.units.selection.system_units, &system_units);
        update_list(&self.content.units.selection.user_units, &user_units);

        let system_units = Arc::new(RwLock::new(system_units));
        let user_units = Arc::new(RwLock::new(user_units));

        self.connect_unit_lists(system_units.clone(), user_units.clone());
        self.connect_enable(system_units.clone(), user_units.clone());
        self.connect_activate(system_units.clone(), user_units.clone());
        self.connect_unit_switch(system_units.clone(), user_units.clone());
        self.connect_refresh_units(system_units.clone(), user_units.clone());
        self.connect_search(system_units, user_units);

        {
            let units = &self.content.units.selection.system_units;
            units.get_row_at_index(0).map(|row| units.select_row(&row));
        }

        // Wrap the `App` within `ConnectedApp` to enable the developer to execute the program.
        ConnectedApp(self)
    }

    fn connect_refresh_units(
        &self,
        system_units: Arc<RwLock<Units>>,
        user_units: Arc<RwLock<Units>>,
    ) {
        let system_list = self.content.units.selection.system_units.clone();
        let user_list = self.content.units.selection.user_units.clone();
        let stack = self.content.units.selection.units_stack.clone();
        self.content.units.selection.refresh.connect_clicked(move |_| {
            let mut system_lock = system_units.write().unwrap();
            let mut user_lock = user_units.write().unwrap();
            let new_system_units = Units::new(Kind::System, Location::Localhost).unwrap();
            let new_user_units = Units::new(Kind::User, Location::Localhost).unwrap();
            update_list(&system_list, &new_system_units);
            update_list(&user_list, &new_user_units);
            *system_lock = new_system_units;
            *user_lock = new_user_units;
            drop(system_lock);
            drop(user_lock);
            if stack_is_user(&stack) {
                user_list.get_row_at_index(0).map(|row| user_list.select_row(&row));
            } else {
                system_list.get_row_at_index(0).map(|row| system_list.select_row(&row));
            }
        });
    }

    fn connect_unit_switch(
        &self,
        system_units: Arc<RwLock<Units>>,
        user_units: Arc<RwLock<Units>>,
    ) {
        let stack = self.content.units.selection.units_stack.clone();
        let switcher = self.content.units.content.notebook.container.clone();
        let journal = self.content.units.content.notebook.journal_buff.clone();
        let dependencies = self.content.units.content.notebook.dependencies_buff.clone();
        let system_list = self.content.units.selection.system_units.clone();
        let user_list = self.content.units.selection.user_units.clone();
        let save = self.content.units.content.file_save.clone();
        let properties = self.content.units.content.notebook.properties.clone();
        let window = self.window.clone();

        switcher.connect_switch_page(move |_, _, page_no| {
            let (kind, list, units) = if stack_is_user(&stack) {
                (Kind::User, &user_list, user_units.read().unwrap())
            } else {
                (Kind::System, &system_list, system_units.read().unwrap())
            };

            let id = match list.get_selected_row() {
                Some(row) => row.get_index(),
                None => {
                    window.error_dialog("unable to get selected row");
                    return;
                }
            };

            let row = &units[id as usize];

            match page_no {
                0 => {
                    save.set_visible(true);
                }
                1 => {
                    save.set_visible(false);
                    systemd::get_journal(kind, &row.name)
                        .map_or_else(|| journal.set_text(""), |text| journal.set_text(&text));
                }
                2 => {
                    save.set_visible(false);
                    dependencies.set_text(&systemd::list_dependencies(kind, &row.name));
                }
                3 => {
                    save.set_visible(false);
                    properties.get_children().iter().for_each(|c| c.destroy());
                    systemd::list_properties(kind, &row.name, |id, p, v| fill_property(&properties, id, p, v));
                    properties.show_all();
                }
                _ => (),
            }
        });
    }

    fn connect_search(&self, system_units: Arc<RwLock<Units>>, user_units: Arc<RwLock<Units>>) {
        let system_list = self.content.units.selection.system_units.clone();
        let user_list = self.content.units.selection.user_units.clone();
        let stack = self.content.units.selection.units_stack.clone();
        self.content.units.selection.search.connect_search_changed(move |search| {
            if let Some(text) = search.get_text() {
                let (list, units) = if stack_is_user(&stack) {
                    (&user_list, user_units.read().unwrap())
                } else {
                    (&system_list, system_units.read().unwrap())
                };

                units.iter().enumerate().for_each(|(index, unit)| {
                    let visibility = unit.name.contains(&text);
                    list.get_row_at_index(index as i32).map(|w| w.set_visible(visibility));
                });
            }
        });
    }

    fn connect_enable(&self, system_units: Arc<RwLock<Units>>, user_units: Arc<RwLock<Units>>) {
        let system_list = self.content.units.selection.system_units.clone();
        let user_list = self.content.units.selection.user_units.clone();
        let stack = self.content.units.selection.units_stack.clone();
        let window = self.window.clone();

        self.content.units.content.enabled.connect_clicked(move |enabled| {
            let (kind, list, mut units) = if stack_is_user(&stack) {
                (Kind::User, &user_list, user_units.write().unwrap())
            } else {
                (Kind::System, &system_list, system_units.write().unwrap())
            };

            let id = match list.get_selected_row() {
                Some(row) => row.get_index(),
                None => {
                    window.error_dialog("unable to get selected row");
                    return;
                }
            };

            let is_enabled = enabled.get_label().map_or(false, |enabled| enabled == "Disable");
            let row: Option<&mut systemd::Unit> = units.deref_mut().get_mut(id as usize);
            row.map(|row| {
                match row.toggle_enablement(kind, Location::Localhost, is_enabled) {
                    Ok(()) => update_enable_button(&enabled, row.status),
                    Err(why) => window.error_dialog(&why.to_string())
                }
            });
        });
    }

    fn connect_activate(&self, system_units: Arc<RwLock<Units>>, user_units: Arc<RwLock<Units>>) {
        let system_list = self.content.units.selection.system_units.clone();
        let user_list = self.content.units.selection.user_units.clone();
        let stack = self.content.units.selection.units_stack.clone();
        let window = self.window.clone();

        self.content.units.content.active.connect_clicked(move |active| {
            let (kind, list, mut units) = if stack_is_user(&stack) {
                (Kind::User, &user_list, user_units.write().unwrap())
            } else {
                (Kind::System, &system_list, system_units.write().unwrap())
            };

            let id = match list.get_selected_row() {
                Some(row) => row.get_index(),
                None => {
                    window.error_dialog("unable to select row");
                    return;
                }
            };

            let is_active = active.get_label().map_or(false, |active| active == "Stop");
            let row: Option<&mut systemd::Unit> = units.deref_mut().get_mut(id as usize);
            row.map(|row| {
                match row.toggle_activeness(kind, Location::Localhost, is_active) {
                    Ok(()) => update_active_button(&active, row.active),
                    Err(why) => window.error_dialog(&why.to_string())
                }
            });
        });
    }

    fn connect_unit_lists(&self, system_units: Arc<RwLock<Units>>, user_units: Arc<RwLock<Units>>) {
        self.select_unit(Kind::System, system_units);
        self.select_unit(Kind::User, user_units);
    }

    fn select_unit(&self, kind: Kind, units: Arc<RwLock<Units>>) {
        let listbox = if kind == Kind::User {
            &self.content.units.selection.user_units
        } else {
            &self.content.units.selection.system_units
        };
        let active = self.content.units.content.active.clone();
        let enabled = self.content.units.content.enabled.clone();
        let file = self.content.units.content.notebook.file_buff.clone();
        let journal = self.content.units.content.notebook.journal_buff.clone();
        let dependencies = self.content.units.content.notebook.dependencies_buff.clone();
        let save = self.content.units.content.file_save.clone();
        let description = self.content.units.content.description.clone();
        let switcher = self.content.units.content.notebook.container.clone();
        let properties = self.content.units.content.notebook.properties.clone();

        listbox.connect_row_selected(move |_, row| {
            let id = match row.as_ref() {
                Some(row) => row.get_index(),
                None => return,
            };

            let units = units.read().unwrap();
            let row = &units[id as usize];

            update_active_button(&active, row.active);
            update_enable_button(&enabled, row.status);

            match switcher.get_current_page().unwrap_or(0) {
                0 => {
                    save.set_visible(true);
                    match systemd::get_file(kind, &row.name) {
                        Some((_path, contents)) => {
                            description.set_text(
                                systemd::get_unit_description(&contents)
                                    .unwrap_or("No Description"),
                            );
                            file.set_text(&contents)
                        }
                        None => {
                            file.set_text("");
                            description.set_text("");
                        }
                    }
                }
                1 => {
                    save.set_visible(false);
                    systemd::get_journal(kind, &row.name)
                        .map_or_else(|| journal.set_text(""), |text| journal.set_text(&text));
                    match systemd::get_file(kind, &row.name) {
                        Some((_path, contents)) => {
                            description.set_text(
                                systemd::get_unit_description(&contents)
                                    .unwrap_or("No Description"),
                            );
                            file.set_text(&contents)
                        }
                        None => {
                            file.set_text("");
                            description.set_text("");
                        }
                    }
                }
                2 => {
                    save.set_visible(false);
                    dependencies.set_text(&systemd::list_dependencies(kind, &row.name));
                    match systemd::get_file(kind, &row.name) {
                        Some((_path, contents)) => {
                            description.set_text(
                                systemd::get_unit_description(&contents)
                                    .unwrap_or("No Description"),
                            );
                            file.set_text(&contents)
                        }
                        None => {
                            file.set_text("");
                            description.set_text("");
                        }
                    }
                }
                3 => {
                    save.set_visible(false);
                    match systemd::get_file(kind, &row.name) {
                        Some((_path, contents)) => {
                            description.set_text(
                                systemd::get_unit_description(&contents)
                                    .unwrap_or("No Description"),
                            );
                            file.set_text(&contents)
                        }
                        None => {
                            file.set_text("");
                            description.set_text("");
                        }
                    }
                    properties.get_children().iter().for_each(|c| c.destroy());
                    systemd::list_properties(kind, &row.name, |id, p, v| fill_property(&properties, id, p, v));
                    properties.show_all();
                }
                _ => (),
            }
        });
    }
}

fn fill_property(properties: &Grid, id: i32, property: &str, value: &str) {
    let buffer = TextBuffer::new(None);
    buffer.set_text(property);
    let property = TextView::new_with_buffer(&buffer);
    property.set_editable(false);

    let buffer = TextBuffer::new(None);
    buffer.set_text(value);
    let value = TextView::new_with_buffer(&buffer);
    value.set_wrap_mode(WrapMode::Word);
    value.set_editable(false);

    property.get_style_context().map(
        |p| value.get_style_context().map(|v| {
            let class = if id % 2 == 0 {
                "even_row"
            } else {
                "odd_row"
            };
            p.add_class(class);
            v.add_class(class);
        })
    );

    properties.attach(&property, 0, id, 1, 1);
    properties.attach(&value, 1, id, 1, 1);
}

fn stack_is_user(stack: &Stack) -> bool {
    stack.get_visible_child_name().map_or(false, |name| &name == "User")
}

fn update_button(button: &Button, label: &str, remove_class: &str, add_class: &str) {
    button.set_label(label);
    button.get_style_context().map(|c| {
        c.add_class(add_class);
        c.remove_class(remove_class);
    });
}

fn update_active_button(active: &Button, is_active: bool) {
    if is_active {
        update_button(active, "Stop", SUGGESTED, DESTRUCTIVE);
    } else {
        update_button(active, "Start", DESTRUCTIVE, SUGGESTED);
    }
}

fn update_enable_button(enabled: &Button, status: UnitStatus) {
    let sensitive = match status {
        UnitStatus::Disabled => {
            update_button(enabled, "Enable", DESTRUCTIVE, SUGGESTED);
            true
        }
        UnitStatus::Enabled => {
            update_button(enabled, "Disable", SUGGESTED, DESTRUCTIVE);
            true
        }
        UnitStatus::Masked => false,
    };

    enabled.set_sensitive(sensitive);
}

fn update_list(units: &ListBox, new_items: &[systemd::Unit]) {
    units.get_children().into_iter().for_each(|widget| widget.destroy());
    new_items.into_iter().for_each(|item| {
        let label = Label::new(item.name.as_str());
        label.set_halign(Align::Start);
        label.set_margin_left(5);
        label.set_margin_right(15);
        units.insert(&label, -1);
    });
    units.show_all();
}
