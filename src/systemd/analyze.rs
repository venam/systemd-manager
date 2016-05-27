use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub struct Analyze {
    pub time: u32,
    pub service: String,
}

impl Analyze {
    /// Returns the results of `systemd-analyze blame` as a vector of `Analyze` units
    pub fn blame() -> Vec<Analyze> {
        String::from_utf8(Command::new("systemd-analyze").arg("blame").output().unwrap().stdout).unwrap()
            .lines().rev().map(|x| parse_blame(x)).collect::<Vec<Analyze>>()
    }

    /// Returns the results of `systemd-analyze time` as three `String` values (`kernel`, `userspace`, `total`)
    pub fn time() -> (String, String, String) {
        let stdout = String::from_utf8(Command::new("systemd-analyze").arg("time").output().unwrap().stdout).unwrap();
        let mut stdout = stdout.split_whitespace();
        let kernel = String::from(stdout.nth(3).unwrap_or("N/A"));
        let userspace = String::from(stdout.nth(2).unwrap_or("N/A"));
        let total = String::from(stdout.nth(2).unwrap_or("N/A"));
        (kernel, userspace, total)
    }
}

/// Parses the stdout of an individual line of the `systemd-analyze blame` command and returns it as an `Analyze` unit.
fn parse_blame(x: &str) -> Analyze {
    let mut values: Vec<&str> = x.trim().split_whitespace().collect();
    let service = values.pop().unwrap();
    let time = values.iter().fold(0u32, |acc, x| acc + parse_time(x));
    Analyze {
        time: time,
        service: String::from(service)
    }
}

/// Parses a unit of a time in milliseconds
fn parse_time(input: &str) -> u32 {
    if input.ends_with("ms") {
        input[0..input.len()-2].parse::<u32>().unwrap_or(0)
    } else if input.ends_with('s') {
        (input[0..input.len()-1].parse::<f32>().unwrap_or(0f32) * 1000f32) as u32
    } else if input.ends_with("min") {
        input[0..input.len()-3].parse::<u32>().unwrap_or(0) * 60000u32
    } else {
        0u32
    }
}

#[test]
fn test_analyze_minutes() {
    let correct = Analyze{time: 218514, service: String::from("updatedb.service")};
    assert_eq!(correct, parse_blame("3min 38.514s updatedb.service"));
}

#[test]
fn test_analyze_seconds() {
    let correct = Analyze{time: 15443, service: String::from("openntpd.service")};
    assert_eq!(correct, parse_blame("15.443s openntpd.service"));
}

#[test]
fn test_analyze_milliseconds() {
    let correct = Analyze{time: 1989, service: String::from("systemd-sysctl.service")};
    assert_eq!(correct, parse_blame("1989ms systemd-sysctl.service"));
}
