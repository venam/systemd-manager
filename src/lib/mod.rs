extern crate dbus;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate quickersort;

pub mod analyze;
pub mod dbus_interface;
mod unit;
mod units;

use self::dbus_interface::*;
pub use self::unit::{Unit, UnitError, UnitStatus};
pub use self::units::Units;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Kind {
    User,
    System,
}

#[derive(Debug, PartialEq)]
pub enum Location {
    Localhost,
    External(String),
}

/// Obtain the description from the unit file and return it.
pub fn get_unit_description(info: &str) -> Option<&str> {
    info.lines()
        // Find the line that starts with `Description=`.
        .find(|x| x.starts_with("Description="))
        // Split the line and return the latter half that contains the description.
        .map(|description| description.split_at(12).1)
}
