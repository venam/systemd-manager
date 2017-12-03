use super::{Content, Header};
use gtk;
use gtk::*;
use std::ops::DerefMut;
use std::process;
use std::sync::{Arc, RwLock};
use systemd::{self, Kind, Location, UnitStatus, Units};

const DESTRUCTIVE: &str = "destructive-action";
const SUGGESTED: &str = "suggested-action";

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
        // Create a the headerbar and it's associated content.
        let header = Header::new();
        // Create the content container and all of it's widgets.
        let content = Content::new();

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


        self.content.units.selection.update_list(Kind::System, &system_units);
        self.content.units.selection.update_list(Kind::User, &user_units);

        let system_units = Arc::new(RwLock::new(system_units));
        let user_units = Arc::new(RwLock::new(user_units));

        self.connect_units_list(system_units.clone(), user_units.clone());
        self.connect_enable(system_units.clone(), user_units.clone());
        self.connect_activate(system_units.clone(), user_units.clone());
        self.connect_unit_switch(system_units.clone(), user_units.clone());
        self.connect_search(system_units, user_units);

        // Wrap the `App` within `ConnectedApp` to enable the developer to execute the program.
        ConnectedApp(self)
    }

    fn connect_unit_switch(&self, system_units: Arc<RwLock<Units>>, user_units: Arc<RwLock<Units>>) {
        let stack = self.content.units.selection.units_stack.clone();
        let switcher = self.content.units.content.notebook.container.clone();
        let file = self.content.units.content.notebook.file_buff.clone();
        let journal = self.content.units.content.notebook.journal_buff.clone();
        let dependencies = self.content.units.content.notebook.dependencies_buff.clone();
        let system_list = self.content.units.selection.system_units.clone();
        let user_list = self.content.units.selection.user_units.clone();
        switcher.connect_switch_page(move |_, _, page_no| {
            let (kind, list, units) = if stack_is_user(&stack) {
                (Kind::User, &user_list, user_units.read().unwrap())
            } else {
                (Kind::System, &system_list, system_units.read().unwrap())
            };

            let id = match list.get_selected_row() {
                Some(row) => row.get_index(),
                None => {
                    eprintln!("invalid row");
                    return;
                }
            };

            let row = &units[id as usize];

            match page_no {
                0 => {
                    match systemd::get_file(kind, &row.name) {
                        Some((_path, contents)) => file.set_text(&contents),
                        None => file.set_text(""),
                    }
                }
                1 => {
                    match systemd::get_journal(kind, &row.name) {
                        Some(text) => journal.set_text(&text),
                        None => journal.set_text("")
                    }
                }
                _ => ()
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
        self.content.units.content.enabled.connect_clicked(move |enabled| {
            let (kind, list, mut units) = if stack_is_user(&stack) {
                (Kind::User, &user_list, user_units.write().unwrap())
            } else {
                (Kind::System, &system_list, system_units.write().unwrap())
            };

            let id = match list.get_selected_row() {
                Some(row) => row.get_index(),
                None => {
                    eprintln!("invalid row");
                    return;
                }
            };

            let is_enabled = enabled.get_label().map_or(false, |enabled| enabled == "Disable");
            let row: Option<&mut systemd::Unit> = units.deref_mut().get_mut(id as usize);
            row.map(|row| {
                if row.toggle_enablement(kind, Location::Localhost, is_enabled).is_ok() {
                    update_enable_button(&enabled, row.status);
                }
            });
        });
    }

    fn connect_activate(&self, system_units: Arc<RwLock<Units>>, user_units: Arc<RwLock<Units>>) {
        let system_list = self.content.units.selection.system_units.clone();
        let user_list = self.content.units.selection.user_units.clone();
        let stack = self.content.units.selection.units_stack.clone();
        self.content.units.content.active.connect_clicked(move |active| {
            let (kind, list, mut units) = if stack_is_user(&stack) {
                (Kind::User, &user_list, user_units.write().unwrap())
            } else {
                (Kind::System, &system_list, system_units.write().unwrap())
            };

            let id = match list.get_selected_row() {
                Some(row) => row.get_index(),
                None => {
                    eprintln!("invalid row");
                    return;
                }
            };

            let is_active = active.get_label().map_or(false, |active| active == "Stop");
            let row: Option<&mut systemd::Unit> = units.deref_mut().get_mut(id as usize);
            row.map(|row| {
                if row.toggle_activeness(kind, Location::Localhost, is_active).is_ok() {
                    update_active_button(&active, row.active);
                }
            });
        });
    }

    fn connect_units_list(&self, system_units: Arc<RwLock<Units>>, user_units: Arc<RwLock<Units>>) {
        let active = self.content.units.content.active.clone();
        let enabled = self.content.units.content.enabled.clone();
        let file_buff = self.content.units.content.notebook.file_buff.clone();
        let description = self.content.units.content.description.clone();
        self.content.units.selection.system_units.connect_row_selected(move |_, row| {
            let id = match row.as_ref() {
                Some(row) => row.get_index(),
                None => return,
            };

            let units = system_units.read().unwrap();
            let row = &units[id as usize];

            update_active_button(&active, row.active);
            update_enable_button(&enabled, row.status);

            match systemd::get_file(Kind::System, &row.name) {
                Some((_path, contents)) => {
                    description.set_text(
                        systemd::get_unit_description(&contents).unwrap_or("No Description"),
                    );

                    file_buff.set_text(&contents);
                }
                None => {
                    description.set_text("Unable to get unit file");
                    file_buff.set_text("");
                }
            }
        });

        let active = self.content.units.content.active.clone();
        let enabled = self.content.units.content.enabled.clone();
        let file_buff = self.content.units.content.notebook.file_buff.clone();
        let description = self.content.units.content.description.clone();
        self.content.units.selection.user_units.connect_row_selected(move |_, row| {
            let id = match row.as_ref() {
                Some(row) => row.get_index(),
                None => return,
            };

            let units = user_units.read().unwrap();
            let row = &units[id as usize];

            update_active_button(&active, row.active);
            update_enable_button(&enabled, row.status);

            match systemd::get_file(Kind::User, &row.name) {
                Some((_path, contents)) => {
                    description.set_text(
                        systemd::get_unit_description(&contents).unwrap_or("No Description"),
                    );

                    file_buff.set_text(&contents);
                }
                None => {
                    description.set_text("Unable to get unit file");
                    file_buff.set_text("");
                }
            }
        });
    }
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
