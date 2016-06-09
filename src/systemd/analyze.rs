use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub struct Analyze {
    pub time: u32,
    pub service: String,
}

impl Analyze {
    /// Returns the results of `systemd-analyze blame` as a vector of `Analyze` units
    pub fn blame() -> Option<Vec<Analyze>> {
        Command::new("systemd-analyze").arg("blame").output().map(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or(None).and_then(|stdout| map_blames(stdout.as_str()))
    }

    /// Returns the results of `systemd-analyze time` as three `String` values (`kernel`, `userspace`, `total`)
    pub fn time() -> (String, String, String) {
        Command::new("systemd-analyze").arg("time").output().map(|output| String::from_utf8(output.stdout).ok())
            .unwrap_or(None).map_or(("N/A".to_owned(), "N/A".to_owned(), "N/A".to_owned()), |stdout| {
                map_times(stdout.as_str())
            })
    }
}

/// Take the stdout of `systemd-analyze blame` and map the values to a vector of Analyze units.
fn map_blames(stdout: &str) -> Option<Vec<Analyze>> {
    let mut output: Vec<Analyze> = Vec::new();
    for item in stdout.lines().rev() {
        match parse_blame(item) {
            Some(item) => output.push(item),
            None       => return None
        }
    }
    Some(output)
}

/// Take the stdout of `systemd-analyze time` and map the values in the string.
fn map_times(stdout: &str) -> (String, String, String) {
    let mut stdout = stdout.split_whitespace();
    let kernel     = String::from(stdout.nth(3).unwrap_or("N/A"));
    let userspace  = String::from(stdout.nth(2).unwrap_or("N/A"));
    let total      = String::from(stdout.nth(2).unwrap_or("N/A"));
    (kernel, userspace, total)
}

/// Parses the stdout of an individual line of the `systemd-analyze blame` command and returns it as an `Analyze` unit.
fn parse_blame(x: &str) -> Option<Analyze> {
    let mut values: Vec<&str> = x.trim().split_whitespace().collect();
    values.pop().map(|service| {
        let time = values.iter().fold(0u32, |acc, x| acc + parse_time(x));
        Analyze { time: time, service: String::from(service) }
    })

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
fn test_map_times() {
    let example = "Startup finished in 7.621s (kernel) + 23.949s (userspace) = 31.571s";
    assert_eq!((String::from("7.621s"), String::from("23.949s"), String::from("31.571s")), map_times(example));
}

#[test]
fn test_analyze_minutes() {
    let correct = Analyze{time: 218514, service: String::from("updatedb.service")};
    assert_eq!(Some(correct), parse_blame("3min 38.514s updatedb.service"));
}

#[test]
fn test_analyze_seconds() {
    let correct = Analyze{time: 15443, service: String::from("openntpd.service")};
    assert_eq!(Some(correct), parse_blame("15.443s openntpd.service"));
}

#[test]
fn test_analyze_milliseconds() {
    let correct = Analyze{time: 1989, service: String::from("systemd-sysctl.service")};
    assert_eq!(Some(correct), parse_blame("1989ms systemd-sysctl.service"));
}

#[test]
fn test_analyze_garbage_input() {
    assert_eq!(None, parse_blame(""))
}
