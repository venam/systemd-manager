mod dbus_interface;

use quickersort;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use self::dbus_interface::*;

#[derive(Debug)]
pub enum UnitError {
    InvalidStatus,
    MissingInformation,
}

pub struct Units(Vec<Unit>);

impl Deref for Units {
    type Target = [Unit];

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Units {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Units {
    pub fn new(kind: Kind, from: Location) -> io::Result<Units> {
        let mut units = Vec::new();

        let output = match (kind, &from) {
            (Kind::System, &Location::Localhost) => Command::new("systemctl")
                .arg("list-unit-files")
                .arg("--state")
                .arg("enabled,disabled,masked")
                .output()?,
            (Kind::User, &Location::Localhost) => Command::new("systemctl")
                .arg("list-unit-files")
                .arg("--user")
                .arg("--state")
                .arg("enabled,disabled,masked")
                .output()?,
            _ => unimplemented!(),
        };

        let output = String::from_utf8(output.stdout).unwrap();

        for line in output.lines().skip(1).take_while(|line| !line.is_empty()) {
            let info = match line.parse::<Unit>() {
                Ok(info) => info,
                Err(UnitError::InvalidStatus) => continue,
                Err(UnitError::MissingInformation) => {
                    eprintln!("systemd-manager: missing info: {}", line);
                    continue;
                }
            };
            units.push(info)
        }

        let output = match (kind, from) {
            (Kind::System, Location::Localhost) => Command::new("systemctl")
                .arg("is-active")
                .args(&units.iter().map(|x| x.name.as_str()).collect::<Vec<&str>>())
                .output()?,
            (Kind::User, Location::Localhost) => Command::new("systemctl")
                .arg("is-active")
                .arg("--user")
                .args(&units.iter().map(|x| x.name.as_str()).collect::<Vec<&str>>())
                .output()?,
            _ => unimplemented!(),
        };

        {
            let mut units_iter = units.iter_mut();
            output.stdout.split(|&b| b == b'\n').map(|line| line.get(0) == Some(&b'a')).for_each(
                |is_active| {
                    if let Some(ref mut unit) = units_iter.next() {
                        unit.active = is_active;
                    }
                },
            )
        }

        quickersort::sort_by(&mut units, &|a, b| a.name.cmp(&b.name));
        Ok(Units(units))
    }
}

#[derive(Debug)]
pub struct Unit {
    pub name:   String,
    pub active: bool,
    pub status: UnitStatus,
}

impl Unit {
    pub fn toggle_enablement(
        &mut self,
        kind: Kind,
        location: Location,
        is_enabled: bool,
    ) -> io::Result<()> {
        if location == Location::Localhost {
            if is_enabled {
                match disable(kind, &self.name) {
                    Ok(()) => self.status = UnitStatus::Disabled,
                    Err(why) => eprintln!("{}", why),
                }
            } else {
                match enable(kind, &self.name) {
                    Ok(()) => self.status = UnitStatus::Enabled,
                    Err(why) => eprintln!("{}", why),
                }
            }
            Ok(())
        } else {
            unimplemented!()
        }
    }

    pub fn toggle_activeness(
        &mut self,
        kind: Kind,
        location: Location,
        is_active: bool,
    ) -> io::Result<()> {
        if location == Location::Localhost {
            if is_active {
                match stop(kind, &self.name) {
                    Ok(()) => self.active = false,
                    Err(why) => eprintln!("{}", why),
                }
            } else {
                match start(kind, &self.name) {
                    Ok(()) => self.active = true,
                    Err(why) => eprintln!("{}", why),
                }
            }
            Ok(())
        } else {
            unimplemented!()
        }
    }
}

impl FromStr for Unit {
    type Err = UnitError;

    fn from_str(data: &str) -> Result<Unit, UnitError> {
        let mut iter = data.split_whitespace();
        if let (Some(unit), Some(status)) = (iter.next(), iter.next()) {
            let status = match status {
                "enabled" => UnitStatus::Enabled,
                "disabled" => UnitStatus::Disabled,
                "masked" => UnitStatus::Masked,
                _ => return Err(UnitError::InvalidStatus),
            };

            Ok(Unit { name: unit.to_owned(), active: false, status })
        } else {
            Err(UnitError::MissingInformation)
        }
    }
}

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

#[derive(Debug, Copy, Clone)]
pub enum UnitStatus {
    Enabled,
    Disabled,
    Masked,
}

pub fn get_file(kind: Kind, name: &str) -> Option<(PathBuf, String)> {
    let cmd = if kind == Kind::System {
        Command::new("systemctl").arg("cat").arg(name).output().ok()
    } else {
        Command::new("systemctl").arg("cat").arg("--user").arg(name).output().ok()
    };

    cmd.and_then(|output| String::from_utf8(output.stdout).ok())
        // Take the contained file and return a `String` of the file contents, else return an empty `String`.
        .and_then(|output| {
            let (path, content) = match output.find('\n') {
                Some(pos) if pos > 3 => output.split_at(pos),
                _ => return None
            };

            Some((Path::new(&path[2..]).into(), content.trim().into()))
        })
}

/// Obtain the description from the unit file and return it.
pub fn get_unit_description(info: &str) -> Option<&str> {
    info.lines()
        // Find the line that starts with `Description=`.
        .find(|x| x.starts_with("Description="))
        // Split the line and return the latter half that contains the description.
        .map(|description| description.split_at(12).1)
}

pub fn list_dependencies(kind: Kind, name: &str) -> String {
    let cmd = if kind == Kind::System {
        Command::new("systemctl").arg("list-dependencies").arg(name).output().ok()
    } else {
        Command::new("systemctl").arg("list-dependencies").arg("--user").arg(name).output().ok()
    };

    cmd.and_then(|output| String::from_utf8(output.stdout).ok())
        // Collect a list of dependencies as a `String`, else return the unit's name.
        .map_or(name.into(), |stdout| {
            // Skip the first line of the output
            stdout.lines().skip(1)
                // Skip the first four characters of each line
                .map(|x| x.chars().skip(4).collect::<String>())
                // Fold each line into a single `String`.
                .fold(String::new(), |acc, x| acc + x.as_str() + "\n")
        })
}

pub fn get_journal(kind: Kind, name: &str) -> Option<String> {
    let cmd = if kind == Kind::System {
        Command::new("journalctl").arg("-b").arg("-r").arg("-u").arg(&name).output().ok()
    } else {
        Command::new("journalctl")
            .arg("--user")
            .arg("-b")
            .arg("-r")
            .arg("-u")
            .arg(&name)
            .output()
            .ok()
    };

    cmd.and_then(|output| String::from_utf8(output.stdout).ok())
}