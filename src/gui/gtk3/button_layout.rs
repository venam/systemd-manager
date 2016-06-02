/// This module is used to determine whether the GTK window control layout is set to left or right.
use std::process::Command;

#[derive(Debug, PartialEq)]
/// Signifies that the button layout is either `Left` or `Right`
pub enum ButtonLayout { Left, Right }

/// Uses the `gsettings` command to determine whether the window controls are set to the left or right.
pub fn get() -> ButtonLayout {
    match Command::new("gsettings").arg("get").arg("org.gnome.desktop.wm.preferences").arg("button-layout").output() {
        Ok(output) => parse_button_layout(&output.stdout),
        Err(_) => ButtonLayout::Right
    }
}

/// Parses the stdout of the `gsettings` command to determine whether the controls are set to the left or right.
fn parse_button_layout(stdout: &[u8]) -> ButtonLayout {
    let mut left = String::with_capacity(stdout.len());
    for byte in stdout.iter().take_while(|x| **x != b':') {
        left.push(*byte as char);
    }
    if left.contains("close") { ButtonLayout::Left } else { ButtonLayout::Right }
}

#[test]
fn test_parse_button_layout() {
    assert_eq!(parse_button_layout(b"appmenu:close"), ButtonLayout::Right);
    assert_eq!(parse_button_layout(b"close,minimize,maximize:menu"), ButtonLayout::Left);
}
