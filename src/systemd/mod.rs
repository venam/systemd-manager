pub mod analyze;
pub mod dbus;
pub mod systemctl;

use std::path::Path;

#[derive(Clone, Debug)]
pub struct SystemdUnit {
    pub name:  String,
    pub path:  String,
    pub state: UnitState,
    pub utype: UnitType,
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
pub enum UnitState { Bad, Disabled, Enabled, Indirect, Linked, Masked, Static }
impl UnitState {
    /// Takes the string containing the state information from the dbus message and converts it
    /// into a UnitType by matching the first character.
    pub fn new(x: &str) -> UnitState {
        let x_as_chars: Vec<char> = x.chars().skip(6).take_while(|x| *x != '\"').collect();
        match x_as_chars[0] {
            's' => UnitState::Static,
            'd' => UnitState::Disabled,
            'e' => UnitState::Enabled,
            'i' => UnitState::Indirect,
            'l' => UnitState::Linked,
            'm' => UnitState::Masked,
            'b' => UnitState::Bad,
            _   => panic!("Unknown State: {}", x),
        }
    }
}
