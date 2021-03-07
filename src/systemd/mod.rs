pub mod analyze;
pub mod dbus;
pub mod systemctl;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use self::dbus::dbus::BusType as BusType;


#[derive(Clone, Debug)]
pub struct SystemdUnit {
    pub name: String,
    pub path: String,
    pub state: UnitState,
    pub utype: UnitType,
    pub bustype: BusType,
}

impl SystemdUnit {
    /// Read the unit file and return it's contents so that we can display it in the `gtk::TextView`.
    pub fn get_info(&self) -> String {
        File::open(&self.path)
            .ok()
            // Take the contained file and return a `String` of the file contents, else return an empty `String`.
            .map_or(String::new(), |mut file| {
                // Obtain the capacity to create the string with based on the file's metadata.
                let capacity = file.metadata().map(|x| x.len()).unwrap_or(0) as usize;
                // Create a `String` to store the contents of the file.
                let mut output = String::with_capacity(capacity);
                // Attempt to read the file to the `String`, or return an empty `String` if it fails.
                file.read_to_string(&mut output)
                    .map(|_| output)
                    .ok()
                    .unwrap_or_default()
            })
    }

    /// Obtains the journal log for the given unit.
    pub fn get_journal(&self) -> String {
        Command::new("journalctl")
            .arg(match self.bustype {
                BusType::Session => "--user",
                _ => ""
            })
            .arg("-b")
            .arg("-r")
            .arg("-u")
            .arg(&self.name)
            .output()
            .ok()
            // Collect the output of the journal as a `String`
            .and_then(|output| String::from_utf8(output.stdout).ok())
            // Return the contents of the journal, otherwise return an error message
            .unwrap_or_else(|| format!("Unable to read the journal entry for {}.", self.name))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitType {
    Automount,
    Busname,
    Mount,
    Path,
    Scope,
    Service,
    Slice,
    Socket,
    Swap,
    Target,
    Timer,
}
impl UnitType {
    /// Takes the pathname of the unit as input to determine what type of unit it is.
    pub fn new(pathname: &str) -> UnitType {
        match Path::new(pathname).extension().unwrap().to_str().unwrap() {
            "automount" => UnitType::Automount,
            "busname" => UnitType::Busname,
            "mount" => UnitType::Mount,
            "path" => UnitType::Path,
            "scope" => UnitType::Scope,
            "service" => UnitType::Service,
            "slice" => UnitType::Slice,
            "socket" => UnitType::Socket,
            "swap" => UnitType::Swap,
            "target" => UnitType::Target,
            "timer" => UnitType::Timer,
            _ => panic!("Unknown Type: {}", pathname),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitState {
    Bad,
    Disabled,
    Enabled,
    Generated,
    Indirect,
    Linked,
    Masked,
    Static,
    Transient,
    Alias,
}
impl UnitState {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn new(x: &str) -> UnitState {
        match x.chars().skip(6).take_while(|x| *x != '\"').next().unwrap() {
            's' => UnitState::Static,
            'd' => UnitState::Disabled,
            'e' => UnitState::Enabled,
            'i' => UnitState::Indirect,
            'l' => UnitState::Linked,
            'm' => UnitState::Masked,
            'b' => UnitState::Bad,
            'g' => UnitState::Generated,
            't' => UnitState::Transient,
            'a' => UnitState::Alias,
            _ => panic!("Unknown State: {}", x),
        }
    }
}

/// Obtain the description from the unit file and return it.
pub fn get_unit_description(info: &str) -> Option<&str> {
    info.lines()
        // Find the line that starts with `Description=`.
        .find(|x| x.starts_with("Description="))
        // Split the line and return the latter half that contains the description.
        .map(|description| description.split_at(12).1)
}

/// Returns true if the given `UnitType` and `UnitState` indicates that the unit can be toggled.
fn is_togglable(utype: &UnitType, ustate: &UnitState, wanted_type: &UnitType) -> bool {
    utype == wanted_type && (ustate == &UnitState::Enabled || ustate == &UnitState::Disabled)
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing services which can be enabled and
/// disabled, which are also not templates.
pub fn collect_togglable_services(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            is_togglable(&x.utype, &x.state, &UnitType::Service) && !x.path.ends_with("@.service")
        })
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing sockets which can be enabled and
/// disabled, which are also not templates.
pub fn collect_togglable_sockets(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            is_togglable(&x.utype, &x.state, &UnitType::Socket) && !x.path.ends_with("@.socket")
        })
        .cloned()
        .collect()
}

/// Takes a `Vec<SystemdUnit>` as input and returns a new vector only containing timers which can be enabled and
/// disabled, which are also not templates.
pub fn collect_togglable_timers(units: &[SystemdUnit]) -> Vec<SystemdUnit> {
    units
        .iter()
        .filter(|x| {
            is_togglable(&x.utype, &x.state, &UnitType::Timer) && !x.path.ends_with("@.timer")
        })
        .cloned()
        .collect()
}

#[test]
fn test_get_unit_description() {
    let input = "Description=Name of Service";
    assert_eq!(get_unit_description(input), Some("Name of Service"));
    let input = "No Description";
    assert_eq!(get_unit_description(input), None);
}

#[test]
fn test_is_togglable() {
    let static_state = &UnitState::Static;
    let enabled_state = &UnitState::Enabled;
    let disabled_state = &UnitState::Disabled;
    let service_type = &UnitType::Service;
    let socket_type = &UnitType::Socket;
    let timer_type = &UnitType::Timer;
    assert_eq!(
        is_togglable(service_type, static_state, &UnitType::Service),
        false
    );
    assert_eq!(
        is_togglable(service_type, enabled_state, &UnitType::Service),
        true
    );
    assert_eq!(
        is_togglable(socket_type, disabled_state, &UnitType::Socket),
        true
    );
    assert_eq!(
        is_togglable(timer_type, disabled_state, &UnitType::Timer),
        true
    );
}
