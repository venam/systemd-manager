extern crate dbus;
extern crate quickersort;
use super::dbus::dbus::MessageItem;
use super::{SystemdUnit, UnitType, UnitState};
use std::path::Path;

/// Takes a systemd dbus function as input and returns the result as a `dbus::Message`.
macro_rules! dbus_message {
    ($function:expr) => {{
        let dest      = "org.freedesktop.systemd1";
        let node      = "/org/freedesktop/systemd1";
        let interface = "org.freedesktop.systemd1.Manager";
        dbus::Message::new_method_call(dest, node, interface, $function).
            unwrap_or_else(|e| panic!("{}", e))
    }}
}

/// Takes a `dbus::Message` as input and makes a connection to dbus, returning the reply.
macro_rules! dbus_connect {
    ($message:expr) => {
        dbus::Connection::get_private(dbus::BusType::System).unwrap().
            send_with_reply_and_block($message, 4000)
    }
}

pub trait Dbus {
    fn is_enabled(&self) -> bool;
    fn enable(&self) -> Result<bool, String>;
    fn disable(&self) -> Result<bool, String>;
    fn start(&self) -> Option<String>;
    fn stop(&self) -> Option<String>;
}


impl Dbus for SystemdUnit {
    /// Returns the current enablement status of the unit.
    fn is_enabled(&self) -> bool {
        list_unit_files().iter()
            // Find the specific unit that we waant to obtain the status from
            .find(|unit| &unit.path == &self.path)
            // Map the contained value of that unit and return true if the `UnitState` is `Enabled`.
            .map_or(false, |unit| unit.state == UnitState::Enabled)
    }

    /// Takes the unit pathname of a service and enables it via dbus.
    /// If dbus replies with `[Bool(true), Array([], "(sss)")]`, the service is already enabled.
    fn enable(&self) -> Result<bool, String> {
        let mut message = dbus_message!("EnableUnitFiles");
        message.append_items(&[[self.name.as_str()][..].into(), false.into(), true.into()]);
        dbus_connect!(message)
            // Return `Ok(true)` if the unit is already enabled
            .map(|reply| is_enabled(&reply.get_items()))
            // Return `Err` if the unit could not be enabled.
            .map_err(|reply| reply.to_string())
    }

    /// Takes the unit pathname as input and disables it via dbus.
    /// If dbus replies with `[Array([], "(sss)")]`, the service is already disabled.
    fn disable(&self) -> Result<bool, String> {
        let mut message = dbus_message!("DisableUnitFiles");
        message.append_items(&[[self.name.as_str()][..].into(), false.into()]);
        dbus_connect!(message)
            // Return `Ok(true)` if the unit is already disabled
            .map(|reply| is_disabled(&reply.get_items()))
            // Return `Err` if the unit could not be disabled.
            .map_err(|reply| reply.to_string())
    }

    /// Takes a unit name as input and attempts to start it. It returns an error if an error occurs.
    fn start(&self) -> Option<String> {
        let mut message = dbus_message!("StartUnit");
        message.append_items(&[self.name.as_str().into(), "fail".into()]);
        // Return `Some(error)` if the unit could not be started, else return `None`.
        dbus_connect!(message).err().map(|err| err.to_string())
    }

    /// Takes a unit name as input and attempts to stop it.
    fn stop(&self) -> Option<String> {
        let mut message = dbus_message!("StopUnit");
        message.append_items(&[self.name.as_str().into(), "fail".into()]);
        // Return `Some(error)` if the unit could not be stopped, else return `None`.
        dbus_connect!(message).err().map(|err| err.to_string())
    }
}

/// Communicates with dbus to obtain a list of unit files and returns them as a `Vec<SystemdUnit>`.
pub fn list_unit_files() -> Vec<SystemdUnit> {
    let message = dbus_connect!(dbus_message!("ListUnitFiles"))
        .expect("systemd-manager: unable to get dbus message from systemd").get_items();
    parse_message(&format!("{:?}", message))
}

/// Takes the dbus message as input and maps the information to a `Vec<SystemdUnit>`.
fn parse_message(input: &str) -> Vec<SystemdUnit> {
    // The first seven characters and last ten characters must be removed.
    let message: String = input.chars().skip(7).take(input.chars().count()-17).collect();
    // Create a systemd_units vector to store the collected systemd units.
    let mut systemd_units: Vec<SystemdUnit> = Vec::new();
    // Create an iterator from a comma-separated list of systemd unit variable pairs.
    let mut iterator = message.split(',');
    // Loop through each pair of variables pertaining to the current systemd unit.
    while let (Some(path), Some(state)) = (iterator.next(), iterator.next()) {
        // Skip the first fourteen characters and take all characters until '"' is found. This is the filepath.
        let path: String = path.chars().skip(14).take_while(|x| *x != '\"').collect();
        // Obtain the name of the service by using `std::path::Path` to obtain the file name from the path.
        let name: String = String::from(Path::new(&path).file_name().unwrap().to_str().unwrap());
        // The type of the unit is determined based on the extension of the file.
        let utype = UnitType::new(&path);
        // The state of the unit can be determined by the first character in the `state`
        let state = UnitState::new(state);
        // Push the collected information into the `systemd_units` vector.
        systemd_units.push(SystemdUnit{name: name, path: path, state: state, utype: utype});
    }

    // Sort the list of units by their unit names using quickersort and then return the list.
    quickersort::sort_by(&mut systemd_units[..], &|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    systemd_units
}

/// Return true if the message indicates that the unit is already enabled.
fn is_enabled(items: &[MessageItem]) -> bool {
    format!("{:?}", items) == "[Bool(true), Array([], \"(sss)\")]"
}

/// Return true if the message indicates that the unit is already disabled.
fn is_disabled(items: &[MessageItem]) -> bool {
    format!("{:?}", items) == "[Array([], \"(sss)\")]"
}
