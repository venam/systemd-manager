use quickersort;
use std::io;
use std::ops::{Deref, DerefMut};
use std::process::Command;
use super::{Unit, UnitError, Kind, Location};

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