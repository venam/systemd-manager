use std::fs;
use std::io::Write;
use std::path::Path;

use systemd::{self, UnitState, SystemdUnit};
use systemd::systemctl::Systemctl;
use systemd::dbus::{self, Dbus};
use systemd::analyze::Analyze;

mod button_layout;
use self::button_layout::ButtonLayout;

use gtk;
use gtk::prelude::*;
use gdk::enums::key;

/// Updates the status icon for the selected unit
fn update_icon(icon: &gtk::Image, state: bool) {
    if state { icon.set_from_stock("gtk-yes", 4); } else { icon.set_from_stock("gtk-no", 4); }
}

/// Create a `gtk::ListboxRow` and add it to the `gtk::ListBox`, and then add the `gtk::Image` to a vector so that we can later modify
/// it when the state changes.
fn create_row(row: &mut gtk::ListBoxRow, unit: &SystemdUnit, active_icons: &mut Vec<gtk::Image>, enable_icons: &mut Vec<gtk::Image>) {
    let unit_label = gtk::Label::new(Some(Path::new(&unit.name).file_stem().unwrap().to_str().unwrap()));
    unit_label.set_tooltip_text(systemd::get_unit_description(unit.get_info().as_str()));

    let running_state = if unit.is_active() {
        gtk::Image::new_from_stock("gtk-yes", 4)
    } else {
        gtk::Image::new_from_stock("gtk-no", 4)
    };
    running_state.set_tooltip_text(Some("Active Status"));

    let enablement_state = if unit.state == UnitState::Enabled {
        gtk::Image::new_from_stock("gtk-yes", 4)
    } else {
        gtk::Image::new_from_stock("gtk-no", 4)
    };
    enablement_state.set_tooltip_text(Some("Enablement Status"));

    let unit_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    unit_box.pack_start(&unit_label, false, false, 5);
    unit_box.pack_end(&running_state, false, false, 0);
    unit_box.pack_end(&enablement_state, false, false, 0);
    row.add(&unit_box);

    active_icons.push(running_state);
    enable_icons.push(enablement_state);
}

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
fn setup_systemd_analyze(builder: &gtk::Builder) {
    let analyze_tree: gtk::TreeView = builder.get_object("analyze_tree").unwrap();
    let analyze_store = gtk::ListStore::new(&[gtk::Type::U32, gtk::Type::String]);

    // A simple macro for adding a column to the preview tree.
    macro_rules! add_column {
        ($preview_tree:ident, $title:expr, $id:expr) => {{
            let column   = gtk::TreeViewColumn::new();
            let renderer = gtk::CellRendererText::new();
            column.set_title($title);
            column.set_resizable(true);
            column.pack_start(&renderer, true);
            column.add_attribute(&renderer, "text", $id);
            analyze_tree.append_column(&column);
        }}
    }

    add_column!(analyze_store, "Time (ms)", 0);
    add_column!(analyze_store, "Unit", 1);

    let units = Analyze::blame();
    for value in units.clone() {
        analyze_store.insert_with_values(None, &[0, 1], &[&value.time, &value.service]);
    }

    analyze_tree.set_model(Some(&analyze_store));

    let kernel_time:    gtk::Label = builder.get_object("kernel_time_label").unwrap();
    let userspace_time: gtk::Label = builder.get_object("userspace_time_label").unwrap();
    let total_time:     gtk::Label = builder.get_object("total_time_label").unwrap();
    let times                      = Analyze::time();
    kernel_time.set_label(times.0.as_str());
    userspace_time.set_label(times.1.as_str());
    total_time.set_label(times.2.as_str());
}

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
fn update_journal(journal: &gtk::TextView, unit: &SystemdUnit) {
    journal.get_buffer().unwrap().set_text(unit.get_journal().as_str());
}

pub fn launch() {
    gtk::init().unwrap_or_else(|_| panic!("systemd-manager: failed to initialize GTK."));

    let builder = gtk::Builder::new_from_string(include_str!("interface.glade"));
    let window: gtk::Window                    = builder.get_object("main_window").unwrap();
    let unit_stack: gtk::Stack                 = builder.get_object("unit_stack").unwrap();
    let services_list: gtk::ListBox            = builder.get_object("services_list").unwrap();
    let sockets_list: gtk::ListBox             = builder.get_object("sockets_list").unwrap();
    let timers_list: gtk::ListBox              = builder.get_object("timers_list").unwrap();
    let unit_info: gtk::TextView               = builder.get_object("unit_info").unwrap();
    let ablement_switch: gtk::Switch           = builder.get_object("ablement_switch").unwrap();
    let start_button: gtk::Button              = builder.get_object("start_button").unwrap();
    let stop_button: gtk::Button               = builder.get_object("stop_button").unwrap();
    let save_unit_file: gtk::Button            = builder.get_object("save_button").unwrap();
    let unit_menu_label: gtk::Label            = builder.get_object("unit_menu_label").unwrap();
    let unit_popover: gtk::PopoverMenu         = builder.get_object("unit_menu_popover").unwrap();
    let services_button: gtk::Button           = builder.get_object("services_button").unwrap();
    let sockets_button: gtk::Button            = builder.get_object("sockets_button").unwrap();
    let timers_button: gtk::Button             = builder.get_object("timers_button").unwrap();
    let unit_journal: gtk::TextView            = builder.get_object("unit_journal_view").unwrap();
    let header_service_label: gtk::Label       = builder.get_object("header_service_label").unwrap();
    let systemd_menu_label: gtk::Label         = builder.get_object("systemd_menu_label").unwrap();
    let systemd_units_button: gtk::MenuButton  = builder.get_object("systemd_units_button").unwrap();
    let main_window_stack: gtk::Stack          = builder.get_object("main_window_stack").unwrap();
    let systemd_units: gtk::Button             = builder.get_object("systemd_units").unwrap();
    let systemd_analyze: gtk::Button           = builder.get_object("systemd_analyze").unwrap();
    let systemd_menu_popover: gtk::PopoverMenu = builder.get_object("systemd_menu_popover").unwrap();
    let dependencies_view: gtk::TextView       = builder.get_object("dependencies_view").unwrap();
    let left_bar: gtk::HeaderBar               = builder.get_object("left_bar").unwrap();
    let analyze_header: gtk::HeaderBar         = builder.get_object("analyze_bar").unwrap();
    let units_header: gtk::HeaderBar           = builder.get_object("right_bar").unwrap();

    match button_layout::get() {
        ButtonLayout::Right => {
            left_bar.set_show_close_button(false);
            units_header.set_show_close_button(true);
            analyze_header.set_show_close_button(true);
        },
        ButtonLayout::Left => {
            left_bar.set_show_close_button(true);
            units_header.set_show_close_button(false);
            analyze_header.set_show_close_button(false);
        }
    }

    macro_rules! units_menu_clicked {
        ($units_button:ident, $units:ident, $list:ident, $unit_type:expr) => {{
            let label           = unit_menu_label.clone();
            let stack           = unit_stack.clone();
            let popover         = unit_popover.clone();
            let $units          = $units.clone();
            let $list           = $list.clone();
            let unit_info       = unit_info.clone();
            let ablement_switch = ablement_switch.clone();
            let header          = header_service_label.clone();
            let start_button    = start_button.clone();
            let stop_button     = stop_button.clone();
            let dependencies    = dependencies_view.clone();
            $units_button.connect_clicked(move |_| {
                stack.set_visible_child_name($unit_type);
                label.set_text($unit_type);
                popover.set_visible(false);
                $list.select_row(Some(&$list.get_row_at_index(0).unwrap()));
                let unit = &$units[0];
                let info = unit.get_info();
                unit_info.get_buffer().unwrap().set_text(info.as_str());
                ablement_switch.set_active(unit.is_enabled());
                ablement_switch.set_state(ablement_switch.get_active());
                match systemd::get_unit_description(&info) {
                    Some(description) => header.set_label(description),
                    None              => header.set_label(&unit.name)
                }
                if unit.is_active() {
                    start_button.set_visible(false);
                    stop_button.set_visible(true);
                } else {
                    start_button.set_visible(true);
                    stop_button.set_visible(false);
                }

                dependencies.get_buffer().unwrap().set_text(unit.list_dependencies().as_str())
            });
        }}
    }

    // Initializes the units for a given unit list
    macro_rules! initialize_units {
        ($units:ident, $list:ident, $active_icons:ident, $enable_icons:ident) => {{
            for unit in $units.clone() {
                let mut unit_row = gtk::ListBoxRow::new();
                create_row(&mut unit_row, &unit, &mut $active_icons, &mut $enable_icons);
                $list.insert(&unit_row, -1);
            }
        }}
    }

    // Programs the row_selected signal for a given unit list.
    macro_rules! signal_row_selected {
        ($units:ident, $list:ident) => {{
            let $units          = $units.clone();
            let $list           = $list.clone();
            let unit_info       = unit_info.clone();
            let ablement_switch = ablement_switch.clone();
            let header          = header_service_label.clone();
            let stop_button     = stop_button.clone();
            let start_button    = start_button.clone();
            let dependencies    = dependencies_view.clone();
            $list.connect_row_selected(move |_, row| {
                if let Some(row) = row.clone() {
                    let unit        = &$units[row.get_index() as usize];
                    let description = unit.get_info();
                    unit_info.get_buffer().unwrap().set_text(description.as_str());
                    ablement_switch.set_active(unit.is_enabled());
                    ablement_switch.set_state(ablement_switch.get_active());
                    header.set_label(unit.name.as_str());
                    match systemd::get_unit_description(&description) {
                        Some(description) => header.set_label(description),
                        None              => header.set_label(&unit.name)
                    }
                    if unit.is_active() {
                        start_button.set_visible(false);
                        stop_button.set_visible(true);
                    } else {
                        start_button.set_visible(true);
                        stop_button.set_visible(false);
                    }

                    dependencies.get_buffer().unwrap().set_text(unit.list_dependencies().as_str())
                }
            });
        }}
    }

    // Setup the Analyze stack
    setup_systemd_analyze(&builder);

    // Initialize all of the services, sockets, timers and their respective signals.
    let unit_files                 = dbus::list_unit_files();
    let services                   = dbus::collect_togglable_services(&unit_files);
    let sockets                    = dbus::collect_togglable_sockets(&unit_files);
    let timers                     = dbus::collect_togglable_timers(&unit_files);
    let mut services_icons_active  = Vec::new();
    let mut services_icons_enabled = Vec::new();
    let mut sockets_icons_active   = Vec::new();
    let mut sockets_icons_enabled  = Vec::new();
    let mut timers_icons_active    = Vec::new();
    let mut timers_icons_enabled   = Vec::new();
    initialize_units!(services, services_list, services_icons_active, services_icons_enabled);
    initialize_units!(sockets, sockets_list, sockets_icons_active, sockets_icons_enabled);
    initialize_units!(timers, timers_list, timers_icons_active, timers_icons_enabled);
    signal_row_selected!(timers, timers_list);
    signal_row_selected!(services, services_list);
    signal_row_selected!(sockets, sockets_list);
    units_menu_clicked!(services_button, services, services_list, "Services");
    units_menu_clicked!(sockets_button, sockets, sockets_list, "Sockets");
    units_menu_clicked!(timers_button, timers, timers_list, "Timers");

    { // NOTE: Refresh the journal every second
        let services      = services.clone();
        let services_list = services_list.clone();
        let sockets       = sockets.clone();
        let sockets_list  = sockets_list.clone();
        let timers        = timers.clone();
        let timers_list   = timers_list.clone();
        let unit_stack    = unit_stack.clone();
        let unit_journal  = unit_journal.clone();
        gtk::timeout_add_seconds(1, move || {
            let unit = match unit_stack.get_visible_child_name().unwrap().as_str() {
                "Services" => unsafe { services.get_unchecked(services_list.get_selected_row().unwrap().get_index() as usize) },
                "Sockets"  => unsafe { sockets.get_unchecked(sockets_list.get_selected_row().unwrap().get_index() as usize) },
                "Timers"   => unsafe { timers.get_unchecked(timers_list.get_selected_row().unwrap().get_index() as usize) },
                _          => unreachable!()
            };
            update_journal(&unit_journal, unit);
            gtk::Continue(true)
        });
    }

    { // NOTE: Program the Systemd Analyze Button
        let systemd_analyze      = systemd_analyze.clone();
        let main_window_stack    = main_window_stack.clone();
        let systemd_menu_label   = systemd_menu_label.clone();
        let units_header         = units_header.clone();
        let analyze_header       = analyze_header.clone();
        let systemd_units_button = systemd_units_button.clone();
        let popover              = systemd_menu_popover.clone();
        systemd_analyze.connect_clicked(move |_| {
            main_window_stack.set_visible_child_name("Systemd Analyze");
            systemd_menu_label.set_label("Systemd Analyze");
            systemd_units_button.set_visible(false);
            units_header.set_visible(false);
            analyze_header.set_visible(true);
            popover.set_visible(false);
        });
    }

    { // NOTE: Program the Systemd Unit Button
        let systemd_units_button = systemd_units_button.clone();
        let main_window_stack    = main_window_stack.clone();
        let systemd_menu_label   = systemd_menu_label.clone();
        let systemd_units_button = systemd_units_button.clone();
        let popover              = systemd_menu_popover.clone();
        systemd_units.connect_clicked(move |_| {
            main_window_stack.set_visible_child_name("Systemd Units");
            systemd_menu_label.set_label("Systemd Units");
            systemd_units_button.set_visible(true);
            units_header.set_visible(true);
            analyze_header.set_visible(false);
            popover.set_visible(false);
        });
    }

    { // NOTE: Implement the {dis, en}able button
        let services               = services.clone();
        let services_list          = services_list.clone();
        let services_icons_enabled = services_icons_enabled.clone();
        let sockets                = sockets.clone();
        let sockets_list           = sockets_list.clone();
        let sockets_icons_enabled  = sockets_icons_enabled.clone();
        let timers                 = timers.clone();
        let timers_list            = timers_list.clone();
        let timers_icons_enabled   = timers_icons_enabled.clone();
        let unit_stack             = unit_stack.clone();
        ablement_switch.connect_state_set(move |switch, enabled| {
            let (unit, icon) = match unit_stack.get_visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = match services_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&services[index], &services_icons_enabled[index])
                },
                "Sockets" => {
                    let index = match sockets_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&sockets[index], &sockets_icons_enabled[index])
                },
                "Timers" => {
                    let index = match timers_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&timers[index], &timers_icons_enabled[index])
                },
                _ => unreachable!()
            };
            if enabled && !unit.is_enabled() {
                match unit.enable() {
                    Ok(message)  => {
                        println!("{}", message);
                        update_icon(icon, true);
                    },
                    Err(message) => println!("{}", message)
                }
                switch.set_state(true);
            } else if !enabled && unit.is_enabled() {
                match unit.disable() {
                    Ok(message)  => {
                        println!("{}", message);
                        update_icon(icon, false);
                    },
                    Err(message) => println!("{}", message)
                }
                switch.set_state(false);
            }
            gtk::Inhibit(true)
        });
    }

    { // NOTE: Implement the start button
        let services              = services.clone();
        let services_list         = services_list.clone();
        let sockets               = sockets.clone();
        let sockets_list          = sockets_list.clone();
        let timers                = timers.clone();
        let timers_list           = timers_list.clone();
        let services_icons_active = services_icons_active.clone();
        let sockets_icons_active  = sockets_icons_active.clone();
        let timers_icons_active   = timers_icons_active.clone();
        let unit_stack            = unit_stack.clone();
        let start_button          = start_button.clone();
        let stop_button           = stop_button.clone();
        start_button.connect_clicked(move |button| {
            let (unit, icon) = match unit_stack.get_visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = match services_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&services[index], &services_icons_active[index])
                },
                "Sockets" => {
                    let index = match sockets_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&sockets[index], &sockets_icons_active[index])
                },
                "Timers" => {
                    let index = match timers_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&timers[index], &timers_icons_active[index])

                },
                _ => unreachable!()
            };
            match unit.start() {
                Ok(message) => {
                   println!("{}", message);
                   update_icon(icon, true);
                   button.set_visible(false);
                   stop_button.set_visible(true);
               },
               Err(message) => println!("{}", message)
            }
        });
    }

    { // NOTE: Implement the stop button
        let services              = services.clone();
        let services_list         = services_list.clone();
        let sockets               = sockets.clone();
        let sockets_list          = sockets_list.clone();
        let timers                = timers.clone();
        let timers_list           = timers_list.clone();
        let services_icons_active = services_icons_active.clone();
        let sockets_icons_active  = sockets_icons_active.clone();
        let timers_icons_active   = timers_icons_active.clone();
        let unit_stack            = unit_stack.clone();
        let start_button          = start_button.clone();
        let stop_button           = stop_button.clone();
        stop_button.connect_clicked(move |button| {
            let (unit, icon) = match unit_stack.get_visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = match services_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&services[index], &services_icons_active[index])
                },
                "Sockets" => {
                    let index = match sockets_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&sockets[index], &sockets_icons_active[index])
                },
                "Timers" => {
                    let index = match timers_list.get_selected_row() {
                        Some(row) => row.get_index() as usize,
                        None      => 0
                    };
                    (&timers[index], &timers_icons_active[index])

                },
                _ => unreachable!()
            };
            match unit.stop() {
                Ok(message) => {
                    println!("{}", message);
                    update_icon(icon, false);
                    button.set_visible(false);
                    start_button.set_visible(true);
                },
                Err(message) => println!("{}", message)
            }
        });
    }

    { // NOTE: Save Button
        let unit_info     = unit_info.clone();
        let services      = services.clone();
        let services_list = services_list.clone();
        let sockets       = sockets.clone();
        let sockets_list  = sockets_list.clone();
        let timers        = timers.clone();
        let timers_list   = timers_list.clone();
        let unit_stack    = unit_stack.clone();
        save_unit_file.connect_clicked(move |_| {
            let buffer = unit_info.get_buffer().unwrap();
            let start  = buffer.get_start_iter();
            let end    = buffer.get_end_iter();
            let text   = buffer.get_text(&start, &end, true).unwrap();
            let path = match unit_stack.get_visible_child_name().unwrap().as_str() {
                "Services" => &services[services_list.get_selected_row().unwrap().get_index() as usize].path,
                "Sockets"  => &sockets[sockets_list.get_selected_row().unwrap().get_index() as usize].path,
                "Timers"   => &timers[timers_list.get_selected_row().unwrap().get_index() as usize].path,
                _          => unreachable!()
            };
            match fs::OpenOptions::new().write(true).open(&path) {
                Ok(mut file) => {
                    if let Err(message) = file.write(text.as_bytes()) {
                        println!("Unable to write to file: {:?}", message);
                    }
                },
                Err(message) => println!("Unable to open file: {:?}", message)
            }
        });
    }

    window.show_all();

    // Quit the program when the program has been exited
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Define custom actions on keypress
    window.connect_key_press_event(move |_, key| {
        if let key::Escape = key.get_keyval() { gtk::main_quit() }
        gtk::Inhibit(false)
    });

    gtk::main();
}
