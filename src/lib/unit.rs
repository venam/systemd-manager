use super::{Kind, Location, DbusError, start, stop, enable, disable};
use quickersort;
use std::process::Command;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Copy, Clone)]
pub enum UnitStatus {
    Enabled,
    Disabled,
    Masked,
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
    ) -> Result<(), DbusError> {
        if location == Location::Localhost {
            if is_enabled {
                disable(kind, &self.name)?;
                self.status = UnitStatus::Disabled;
            } else {
                enable(kind, &self.name)?;
                self.status = UnitStatus::Enabled;
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
    ) -> Result<(), DbusError> {
        if location == Location::Localhost {
            if is_active {
                stop(kind, &self.name)?;
            } else {
                start(kind, &self.name)?;
            }
            self.active = !is_active;
            Ok(())
        } else {
            unimplemented!()
        }
    }

    pub fn cat(&self, kind: Kind) -> Option<(PathBuf, String)> {
        let cmd = if kind == Kind::System {
            Command::new("systemctl").arg("cat").arg(&self.name).output().ok()
        } else {
            Command::new("systemctl").arg("cat").arg("--user").arg(&self.name).output().ok()
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

    pub fn journal(&self, kind: Kind) -> Option<String> {
        let cmd = if kind == Kind::System {
            Command::new("journalctl").arg("-b").arg("-r").arg("-u").arg(&self.name).output().ok()
        } else {
            Command::new("journalctl")
                .arg("--user")
                .arg("-b")
                .arg("-r")
                .arg("-u")
                .arg(&self.name)
                .output()
                .ok()
        };

        cmd.and_then(|output| String::from_utf8(output.stdout).ok())
    }

    pub fn properties<F: Fn(i32, &str, &str)>(&self, kind: Kind, action: F) -> Option<()> {
        let cmd = if kind == Kind::System {
            Command::new("systemctl").arg("--no-pager").arg("show").arg(&self.name).output()
        } else {
            Command::new("systemctl").arg("--no-pager").arg("--user").arg("show").arg(&self.name).output()
        };

        let output = cmd.ok().and_then(|output| String::from_utf8(output.stdout).ok())?;

        struct Property<'a> {
            property: &'a str,
            value: &'a str,
        }

        let mut properties = Vec::new();
        for line in output.lines() {
            if let Some(pos) = line.find('=') {
                let (property, value) = line.split_at(pos);
                if value.len() > 1 {
                    properties.push(Property { property, value: &value[1..] });
                }
            }
        }

        quickersort::sort_by(&mut properties, &|a, b| a.property.cmp(&b.property));
        properties.into_iter().enumerate().for_each(|(id, p)| action(id as i32, p.property, p.value));

        Some(())
    }

    pub fn dependencies(&self, kind: Kind) -> String {
        let cmd = if kind == Kind::System {
            Command::new("systemctl").arg("list-dependencies").arg(&self.name).output().ok()
        } else {
            Command::new("systemctl").arg("list-dependencies").arg("--user").arg(&self.name).output().ok()
        };

        cmd.and_then(|output| String::from_utf8(output.stdout).ok())
            // Collect a list of dependencies as a `String`, else return the unit's name.
            .map_or(self.name.clone(), |stdout| {
                // Skip the first line of the output
                stdout.lines().skip(1)
                    // Skip the first four characters of each line
                    .map(|x| x.chars().skip(4).collect::<String>())
                    // Fold each line into a single `String`.
                    .fold(String::new(), |acc, x| acc + x.as_str() + "\n")
            })
    }
}

#[derive(Debug)]
pub enum UnitError {
    InvalidStatus,
    MissingInformation,
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