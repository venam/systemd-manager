use std::process::Command;
use super::SystemdUnit;

pub trait Systemctl {
    fn is_active(&self) -> bool;
    fn list_dependencies(&self) -> String;
}

impl Systemctl for SystemdUnit {
    /// Runs the `systemctl status` command and receives it's stdout to determin the active status of the unit.
    fn is_active(&self) -> bool {
        match String::from_utf8(Command::new("systemctl").arg("status").arg(&self.name).output().unwrap().stdout) {
            Ok(stdout) => parse_state(stdout.as_str()),
            Err(_) => false
        }
    }

    fn list_dependencies(&self) -> String {
        match String::from_utf8(Command::new("systemctl").arg("list-dependencies").arg(&self.name).output().unwrap().stdout) {
            Ok(stdout) => {
                stdout.lines().skip(1).map(|x| x.chars().skip(4).collect::<String>())
                    .fold(String::new(), |acc, x| acc + x.as_str() + "\n")
            },
            Err(_) => self.name.clone()
        }
    }
}

/// Parses the stdout of `systemctl status` to determine if the unit is active (true) or inactive (false).
fn parse_state(status: &str) -> bool {
    match status.lines().nth(2) {
        Some(active_line) => {
            if let Some(value) = active_line.trim().split_at(8).1.chars().next() { value == 'a' } else { false }
        },
        None => false
    }
}

#[test]
fn test_parse_state_active() {
    let input = r##"● systemd-networkd.service - Network Service
   Loaded: loaded (/usr/lib/systemd/system/systemd-networkd.service; enabled; vendor preset: enabled)
   Active: active (running) since Wed 2016-05-18 14:13:36 EDT; 12h ago"##;
   assert_eq!(parse_state(input), true);
}

#[test]
fn test_parse_state_inactive() {
    let input = r##"● NetworkManager.service - Network Manager
   Loaded: loaded (/usr/lib/systemd/system/NetworkManager.service; disabled; vendor preset: disabled)
   Active: inactive (dead)"##;
   assert_eq!(parse_state(input), false);
}
