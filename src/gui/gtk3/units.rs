use gtk::{Box, Image, Label, ListBox, ListBoxRow, Orientation};
use gtk::prelude::*;
use systemd::{self, UnitState, SystemdUnit};
use systemd::systemctl::Systemctl;
use std::path::Path;

/// Create a `gtk::ListboxRow` and add it to the `gtk::ListBox`, and then add the `Image` to a vector so that we can later modify
/// it when the state changes.
pub fn create_row(row: &mut ListBoxRow, unit: &SystemdUnit, active_icons: &mut Vec<Image>, enable_icons: &mut Vec<Image>) {
    // Creates the status icons used by the rows.
    fn get_icon(state: bool, tooltip: &str) -> Image {
        let icon = if state { Image::new_from_stock("gtk-yes", 4) } else { Image::new_from_stock("gtk-no", 4) };
        icon.set_tooltip_text(Some(tooltip));
        icon
    }

    // Create the unit label with the extension removed.
    let unit_label = Label::new(Some(Path::new(&unit.name).file_stem().unwrap().to_str().unwrap()));
    unit_label.set_tooltip_text(systemd::get_unit_description(unit.get_info().as_str()));

    // Create the running and enable status icons.
    let running = get_icon(unit.is_active(), "Active Status");
    let enabled = get_icon(unit.state == UnitState::Enabled, "Enablement Status");

    // Create a horizontal box that contains the unit label, running status, and enablement status.
    let unit_box = Box::new(Orientation::Horizontal, 0);
    unit_box.pack_start(&unit_label, false, false, 5);
    unit_box.pack_end(&running, false, false, 0);
    unit_box.pack_end(&enabled, false, false, 0);

    // Add the box as a row in the `ListBox`.
    row.add(&unit_box);

    // Add the icons to the mutable input vectors so that they can be modified by later actions.
    active_icons.push(running);
    enable_icons.push(enabled);
}

/// Obtains the index of the currently-selected row, else returns the default of 0.
fn get_selected_row(list: &ListBox) -> usize {
    list.get_selected_row().map_or(0, |row| row.get_index() as usize)
}

/// Obtains the currently-selected unit.
pub fn get_current_unit<'a>(units: &'a [SystemdUnit], list: &ListBox) -> &'a SystemdUnit {
    &units[get_selected_row(list)]
}

/// Obtains the currently-selected unit and it's associated icon
pub fn get_current_unit_icons<'a>(units: &'a [SystemdUnit], units_box: &ListBox, icons: &'a [Image]) -> (&'a SystemdUnit, &'a Image) {
    let index = get_selected_row(units_box);
    (&units[index], &icons[index])
}
