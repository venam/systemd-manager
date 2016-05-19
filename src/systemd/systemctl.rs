use std::process::Command;

/// Runs the `systemctl status` command and receives it's stdout to determin the active status of the unit.
pub fn unit_is_active(unit: &str) -> bool {
    match String::from_utf8(Command::new("systemctl").arg("status").arg(unit).output().unwrap().stdout) {
        Ok(stdout) => parse_state(stdout.as_str()),
        Err(_) => false
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
