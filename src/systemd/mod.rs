pub mod analyze;
pub mod dbus;
pub mod systemctl;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct SystemdUnit {
    pub name:  String,
    pub path:  String,
    pub state: UnitState,
    pub utype: UnitType,
}

impl SystemdUnit {
    /// Read the unit file and return it's contents so that we can display it in the `gtk::TextView`.
    pub fn get_info(&self) -> String {
        File::open(&self.path).map(|mut file| {
            let mut output = String::with_capacity(file.metadata().map(|x| x.len()).unwrap_or(0) as usize);
            file.read_to_string(&mut output).map(|_| output).ok().unwrap_or_default()
        }).ok().unwrap_or_default()
    }

    /// Obtains the journal log for the given unit.
    pub fn get_journal(&self) -> String {
        Command::new("journalctl").arg("-b").arg("-r").arg("-u").arg(&self.name).output().ok()
            .and_then(|output| String::from_utf8(output.stdout).ok()).unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitType { Automount, Busname, Mount, Path, Scope, Service, Slice, Socket, Swap, Target, Timer }
impl UnitType {
    /// Takes the pathname of the unit as input to determine what type of unit it is.
    pub fn new(pathname: &str) -> UnitType {
        match Path::new(pathname).extension().unwrap().to_str().unwrap() {
            "automount" => UnitType::Automount,
            "busname"   => UnitType::Busname,
            "mount"     => UnitType::Mount,
            "path"      => UnitType::Path,
            "scope"     => UnitType::Scope,
            "service"   => UnitType::Service,
            "slice"     => UnitType::Slice,
            "socket"    => UnitType::Socket,
            "swap"      => UnitType::Swap,
            "target"    => UnitType::Target,
            "timer"     => UnitType::Timer,
            _           => panic!("Unknown Type: {}", pathname),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitState { Bad, Disabled, Enabled, Generated, Indirect, Linked, Masked, Static, Transient }
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
            _   => panic!("Unknown State: {}", x),
        }
    }
}

/// Obtain the description from the unit file and return it.
pub fn get_unit_description(info: &str) -> Option<&str> {
    info.lines().find(|x| x.starts_with("Description=")).map(|description| description.split_at(12).1)
}
