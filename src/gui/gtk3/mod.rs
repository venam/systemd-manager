use std::fs;
use std::io::Write;

use systemd::{self, SystemdUnit};
use systemd::systemctl::Systemctl;
use systemd::dbus::{self, Dbus};

mod analyze;
mod button_layout;
mod units;
use self::button_layout::ButtonLayout;

use gtk::{self, Image};
use gtk::prelude::*;
use gdk::enums::key;

/// Updates the status icon for the selected unit
fn update_icon(icon: &Image, state: bool) {
    if state { icon.set_from_stock("gtk-yes", 4); } else { icon.set_from_stock("gtk-no", 4); }
}

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
fn update_journal(journal: &gtk::TextView, unit: &SystemdUnit) {
    journal.get_buffer().map(|buffer| buffer.set_text(unit.get_journal().as_str()));
}

pub fn launch() {
    gtk::init().unwrap_or_else(|_| panic!("systemd-manager: failed to initialize GTK."));

    // Initialize all of the widgets that this program will be manipulating from the glade file.
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


    {   // Set the window controls to the left if the button layout is `Left`, else set it to the right.
        let layout_boolean = button_layout::get() == ButtonLayout::Right;
        left_bar.set_show_close_button(!layout_boolean);
        units_header.set_show_close_button(layout_boolean);
        analyze_header.set_show_close_button(layout_boolean);
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
            let unit_journal    = unit_journal.clone();
            $units_button.connect_clicked(move |_| {
                stack.set_visible_child_name($unit_type);
                label.set_text($unit_type);
                popover.set_visible(false);
                if let Some(row) = $list.get_row_at_index(0) {
                    $list.select_row(Some(&row));
                    let unit = &$units[row.get_index() as usize];
                    // Obtain information from the unit's file.
                    let info = unit.get_info();
                    // Set the header label as the description if available, or the unit name if not.
                    header.set_label(systemd::get_unit_description(&info).map_or(&unit.name, |desc| desc));
                    // Write the collected information to the unit file's textivew buffer.
                    unit_info.get_buffer().map(|buffer| buffer.set_text(info.as_str()));
                    // Update the dependency list with the list of dependencies for that unit.
                    dependencies.get_buffer().map(|buffer| buffer.set_text(unit.list_dependencies().as_str()));
                    // Update the unit's journal view
                    update_journal(&unit_journal, &unit);
                    // If the unit is enabled, set the state and active status as true.
                    let unit_enabled = unit.is_enabled();
                    ablement_switch.set_active(unit_enabled);
                    ablement_switch.set_state(unit_enabled);
                    // Use the unit active status to determine which button should be currently visible.
                    let status = unit.is_active();
                    start_button.set_visible(!status);
                    stop_button.set_visible(status);
                }
            });
        }}
    }

    // Initializes the units for a given unit list
    macro_rules! initialize_units {
        ($units:ident, $list:ident, $active_icons:ident, $enable_icons:ident) => {{
            for unit in $units.clone() {
                let mut unit_row = gtk::ListBoxRow::new();
                units::create_row(&mut unit_row, &unit, &mut $active_icons, &mut $enable_icons);
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
            let unit_journal    = unit_journal.clone();
            $list.connect_row_selected(move |_, row| {
                if let Some(row) = row.clone() {
                    let unit = &$units[row.get_index() as usize];
                    // Obtain information from the unit's file.
                    let info = unit.get_info();
                    // Set the header label as the description if available, or the unit name if not.
                    header.set_label(systemd::get_unit_description(&info).map_or(&unit.name, |desc| desc));
                    // Write the collected information to the unit file's textivew buffer.
                    unit_info.get_buffer().map(|buffer| buffer.set_text(info.as_str()));
                    // Update the dependency list with the list of dependencies for that unit.
                    dependencies.get_buffer().map(|buffer| buffer.set_text(unit.list_dependencies().as_str()));
                    // Update the unit's journal view
                    update_journal(&unit_journal, &unit);
                    // If the unit is enabled, set the state and active status as true.
                    let unit_enabled = unit.is_enabled();
                    ablement_switch.set_active(unit_enabled);
                    ablement_switch.set_state(unit_enabled);
                    // Use the unit active status to determine which button should be currently visible.
                    let status = unit.is_active();
                    start_button.set_visible(!status);
                    stop_button.set_visible(status);
                }
            });
        }}
    }

    // Setup the Analyze stack
    analyze::setup(&builder);

    // Initialize all of the services, sockets, and timers.
    let unit_files                 = dbus::list_unit_files();
    let services                   = systemd::collect_togglable_services(&unit_files);
    let sockets                    = systemd::collect_togglable_sockets(&unit_files);
    let timers                     = systemd::collect_togglable_timers(&unit_files);
    // Create vectors to contain status icons that can later be manipulated.
    let mut services_icons_active  = Vec::new();
    let mut services_icons_enabled = Vec::new();
    let mut sockets_icons_active   = Vec::new();
    let mut sockets_icons_enabled  = Vec::new();
    let mut timers_icons_active    = Vec::new();
    let mut timers_icons_enabled   = Vec::new();
    // Initialize the rows in each of the ListBoxes.
    initialize_units!(services, services_list, services_icons_active, services_icons_enabled);
    initialize_units!(sockets, sockets_list, sockets_icons_active, sockets_icons_enabled);
    initialize_units!(timers, timers_list, timers_icons_active, timers_icons_enabled);
    // Program what happens when a row is selected for each of the ListBoxes.
    signal_row_selected!(timers, timers_list);
    signal_row_selected!(services, services_list);
    signal_row_selected!(sockets, sockets_list);
    // Program what happens when a menu button is clicked.
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
            if let Some(child) = unit_stack.get_visible_child_name() {
                let unit = match child.as_str() {
                    "Services" => units::get_current_unit(&services, &services_list),
                    "Sockets"  => units::get_current_unit(&sockets, &sockets_list),
                    "Timers"   => units::get_current_unit(&timers, &timers_list),
                    _          => unreachable!()
                };
                update_journal(&unit_journal, unit);
            }
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
            if let Some(child) = unit_stack.get_visible_child_name() {
                let (unit, icon) = match child.as_str() {
                    "Services" => units::get_current_unit_icons(&services, &services_list, &services_icons_enabled),
                    "Sockets"  => units::get_current_unit_icons(&sockets, &sockets_list, &sockets_icons_enabled),
                    "Timers"   => units::get_current_unit_icons(&timers, &timers_list, &timers_icons_enabled),
                    _          => unreachable!()
                };
                if enabled && !unit.is_enabled() {
                    match unit.enable() {
                        Ok(unit_was_enabled) => {
                            if unit_was_enabled {
                                println!("systemd-manager: {} was already enabled", unit.name);
                            } else {
                                println!("systemd-manager: {} has been enabled", unit.name);
                            }
                            update_icon(icon, true);
                        },
                        Err(message) => println!("systemd-manager: {} could not be enabled: {}", unit.name, message)
                    }
                    switch.set_state(true);
                } else if !enabled && unit.is_enabled() {
                    match unit.disable() {
                        Ok(unit_was_disabled) => {
                            if unit_was_disabled {
                                println!("systemd-manager: {} was already disabled", unit.name);
                            } else {
                                println!("systemd-manager: {} has been disabled", unit.name);
                            }
                            update_icon(icon, false);
                        },
                        Err(message) => println!("systemd-manager: {} could not be disabled: {}", unit.name, message)
                    }
                    switch.set_state(false);
                }
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
            if let Some(child) = unit_stack.get_visible_child_name() {
                let (unit, icon) = match child.as_str() {
                    "Services" => units::get_current_unit_icons(&services, &services_list, &services_icons_active),
                    "Sockets"  => units::get_current_unit_icons(&sockets, &sockets_list, &sockets_icons_active),
                    "Timers"   => units::get_current_unit_icons(&timers, &timers_list, &timers_icons_active),
                    _ => unreachable!()
                };
                match unit.start() {
                    None => {
                       println!("systemd-manager: {} successfully started", unit.name);
                       update_icon(icon, true);
                       button.set_visible(false);
                       stop_button.set_visible(true);
                   },
                   Some(error) => println!("systemd-manager: {} failed to start: {}", unit.name, error)
                }
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
            if let Some(child) = unit_stack.get_visible_child_name() {
                let (unit, icon) = match child.as_str() {
                    "Services" => units::get_current_unit_icons(&services, &services_list, &services_icons_active),
                    "Sockets"  => units::get_current_unit_icons(&sockets, &sockets_list, &sockets_icons_active),
                    "Timers"   => units::get_current_unit_icons(&timers, &timers_list, &timers_icons_active),
                    _ => unreachable!()
                };
                match unit.stop() {
                    None => {
                        println!("systemd-manager: {} successfully stopped", unit.name);
                        update_icon(icon, false);
                        button.set_visible(false);
                        start_button.set_visible(true);
                    },
                    Some(error) => println!("systemd-manager: {} failed to stop: {}", unit.name, error)
                }
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
            if let Some(buffer) = unit_info.get_buffer() {
                let start  = buffer.get_start_iter();
                let end    = buffer.get_end_iter();
                if let Some(text) = buffer.get_text(&start, &end, true) {
                    if let Some(child) = unit_stack.get_visible_child_name() {
                        let unit = match child.as_str() {
                            "Services" => units::get_current_unit(&services, &services_list),
                            "Sockets"  => units::get_current_unit(&sockets, &sockets_list),
                            "Timers"   => units::get_current_unit(&timers, &timers_list),
                            _          => unreachable!()
                        };
                        // Open the unit file with write access
                        let status = fs::OpenOptions::new().write(true).open(&unit.path)
                            // Attempt to write to the file, else return an error message.
                            .map(|mut file| file.write(text.as_bytes()));

                        if let Err(message) = status {
                            println!("systemd-manager: unable to save unit file: {}", message.to_string());
                        }
                    }
                }
            }
        });
    }

    // Fix https://github.com/mmstick/systemd-manager/issues/30
    window.set_wmclass ("systemd-manager", "Systemd-manager");
    
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
