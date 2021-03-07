use std::process::Command;
use super::SystemdUnit;
use super::dbus::dbus::BusType as BusType;

pub trait Systemctl {
    /// Runs the `systemctl status` command and receives it's stdout to determin the active status of the unit.
    fn is_active(&self) -> bool;

    /// Runs `systemctl list-dependencies` to obtain a list of dependencies for the given unit.
    fn list_dependencies(&self) -> String;
}

impl Systemctl for SystemdUnit {
    fn is_active(&self) -> bool {
        Command::new("systemctl").arg(match self.bustype {
            BusType::Session => "--user",
            _ => ""
        }).arg("status").arg(&self.name).output().ok()
            // Collect the command's standard output as a `String` and return it as an `Option`.
            .and_then(|output| String::from_utf8(output.stdout).ok())
            // Determine whether the state of the input is active or not.
            .map_or(false, |stdout| parse_state(stdout.as_str()))
    }

    fn list_dependencies(&self) -> String {
        Command::new("systemctl").arg(match self.bustype {
            BusType::Session => "--user",
            _ => ""
        }).arg("list-dependencies").arg(&self.name).output().ok()
            // Collect the command's standard output as a `String` and return it as an `Option`.
            .and_then(|output| String::from_utf8(output.stdout).ok())
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

/// Parses the stdout of `systemctl status` to determine if the unit is active (true) or inactive (false).
fn parse_state(status: &str) -> bool {
    // The second line contains information pertaining to the state.
    status.lines().nth(2)
        // Determines whether the unit is status is active or inactive.
        .map_or_else(|| false, |active_line| {
            // Collect the first letter from the status parameter which is either '[a]ctive' or '[i]nactive'
            active_line.trim().split_at(8).1.chars().next()
                // If the character is `a` then the status is `[a]ctive`.
                .map_or(false, |value| value == 'a')
        })
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
