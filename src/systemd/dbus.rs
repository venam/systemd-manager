extern crate dbus;
extern crate quickersort;
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
    fn enable(&self) -> Result<String, String>;
    fn disable(&self) -> Result<String, String>;
    fn start(&self) -> Result<String, String>;
    fn stop(&self) -> Result<String, String>;
}


impl Dbus for SystemdUnit {
    /// Returns the current enablement status of the unit
    fn is_enabled(&self) -> bool {
        for unit in list_unit_files() {
            if &unit.path == &self.path { return unit.state == UnitState::Enabled; }
        }
        false
    }

    /// Takes the unit pathname of a service and enables it via dbus.
    /// If dbus replies with `[Bool(true), Array([], "(sss)")]`, the service is already enabled.
    fn enable(&self) -> Result<String, String> {
        let mut message = dbus_message!("EnableUnitFiles");
        message.append_items(&[[self.name.as_str()][..].into(), false.into(), true.into()]);
        match dbus_connect!(message) {
            Ok(reply) => {
                if format!("{:?}", reply.get_items()) == "[Bool(true), Array([], \"(sss)\")]" {
                    Ok(format!("{} already enabled", self.name))
                } else {
                    Ok(format!("{} has been enabled", self.name))
                }
            },
            Err(reply) => Err(format!("Error enabling {}:\n{:?}", self.name, reply))
        }
    }

    /// Takes the unit pathname as input and disables it via dbus.
    /// If dbus replies with `[Array([], "(sss)")]`, the service is already disabled.
    fn disable(&self) -> Result<String, String> {
        let mut message = dbus_message!("DisableUnitFiles");
        message.append_items(&[[self.name.as_str()][..].into(), false.into()]);
        match dbus_connect!(message) {
            Ok(reply) => {
                if format!("{:?}", reply.get_items()) == "[Array([], \"(sss)\")]" {
                    Ok(format!("{} is already disabled", self.name))
                } else {
                    Ok(format!("{} has been disabled", self.name))
                }
            },
            Err(reply) => Err(format!("Error disabling {}:\n{:?}", self.name, reply))
        }
    }

    /// Takes a unit name as input and attempts to start it
    fn start(&self) -> Result<String, String> {
        let mut message = dbus_message!("StartUnit");
        message.append_items(&[self.name.as_str().into(), "fail".into()]);
        match dbus_connect!(message) {
            Ok(_) => Ok(format!("{} successfully started", self.name)),
            Err(error) => Err(format!("{} failed to start:\n{:?}", self.name, error))
        }
    }

    /// Takes a unit name as input and attempts to stop it.
    fn stop(&self) -> Result<String, String> {
        let mut message = dbus_message!("StopUnit");
        message.append_items(&[self.name.as_str().into(), "fail".into()]);
        match dbus_connect!(message) {
            Ok(_) => Ok(format!("{} successfully stopped", self.name)),
            Err(error) => Err(format!("{} failed to stop:\n{:?}", self.name, error))
        }
    }
}

/// Communicates with dbus to obtain a list of unit files and returns them as a `Vec<SystemdUnit>`.
pub fn list_unit_files() -> Vec<SystemdUnit> {
    /// Takes the dbus message as input and maps the information to a `Vec<SystemdUnit>`.
    fn parse_message(input: &str) -> Vec<SystemdUnit> {
        let message = {
            let mut output: String = input.chars().skip(7).collect();
            let len = output.len()-10;
            output.truncate(len);
            output
        };

        // This custom loop iterates across two variables at a time. The first variable contains the
        // pathname of the unit, while the second variable contains the state of that unit.
        let mut systemd_units: Vec<SystemdUnit> = Vec::new();
        let mut iterator = message.split(',');
        while let Some(path) = iterator.next() {
            let path: String = path.chars().skip(14).take_while(|x| *x != '\"').collect();
            let name: String = String::from(Path::new(&path).file_name().unwrap().to_str().unwrap());
            let utype = UnitType::new(&path);
            let state = UnitState::new(iterator.next().unwrap());
            systemd_units.push(SystemdUnit{name: name, path: path, state: state, utype: utype});
        }

        quickersort::sort_by(&mut systemd_units[..], &|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
        systemd_units
    }

    let message = dbus_connect!(dbus_message!("ListUnitFiles")).unwrap().get_items();
    parse_message(&format!("{:?}", message))
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing services which can be enabled and
/// disabled.
pub fn collect_togglable_services(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units.iter().filter(|x| x.utype == UnitType::Service && (x.state == UnitState::Enabled ||
        x.state == UnitState::Disabled) && !x.path.starts_with("/etc/") && !x.path.ends_with("@.service")).cloned().collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing sockets which can be enabled and
/// disabled.
pub fn collect_togglable_sockets(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units.iter().filter(|x| x.utype == UnitType::Socket && (x.state == UnitState::Enabled ||
        x.state == UnitState::Disabled) && !x.path.ends_with("@.socket")).cloned().collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing timers which can be enabled and
/// disabled.
pub fn collect_togglable_timers(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units.iter().filter(|x| x.utype == UnitType::Timer && (x.state == UnitState::Enabled ||
        x.state == UnitState::Disabled) && !x.path.ends_with("@.timer")).cloned().collect()
}
