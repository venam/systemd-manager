use dbus::{BusType, Connection, Message};
use super::*;

/// Takes a systemd dbus function as input and returns the result as a `Message`.
macro_rules! dbus_message {
    ($function:expr) => {{
        let dest      = "org.freedesktop.systemd1";
        let node      = "/org/freedesktop/systemd1";
        let interface = "org.freedesktop.systemd1.Manager";
        Message::new_method_call(dest, node, interface, $function)
            .map_err(|why| DbusError::MethodCallError { why })
    }}
}

/// Takes a `Message` as input and makes a connection to dbus, returning the reply.
macro_rules! dbus_connect {
    ($message:expr, $kind:expr) => {
        Connection::get_private(if $kind == Kind::System { BusType::System } else { BusType::Session })
            .map_err(|why| DbusError::Connection { why: format!("{:?}", why) })
            .and_then(
                |c| c.send_with_reply_and_block($message, 30000)
                    .map_err(|why| DbusError::SendErr { why: format!("{:?}", why) })
            )
    }
}

#[derive(Debug, Fail)]
pub enum DbusError {
    #[fail(display = "method call error: {}", why)]
    MethodCallError { why: String },
    #[fail(display = "dbus connection error: {}", why)]
    Connection { why: String },
    #[fail(display = "dbus send error: {}", why)]
    SendErr { why: String },
}

pub fn enable(kind: Kind, unit: &str) -> Result<(), DbusError> {
    let mut message = dbus_message!("EnableUnitFiles")?;
    message.append_items(&[[unit][..].into(), false.into(), true.into()]);
    dbus_connect!(message, kind).map(|_| ())
}

pub fn disable(kind: Kind, unit: &str) -> Result<(), DbusError> {
    let mut message = dbus_message!("DisableUnitFiles")?;
    message.append_items(&[[unit][..].into(), false.into()]);
    dbus_connect!(message, kind).map(|_| ())
}

pub fn start(kind: Kind, unit: &str) -> Result<(), DbusError> {
    let mut message = dbus_message!("StartUnit")?;
    message.append_items(&[unit.into(), "fail".into()]);
    dbus_connect!(message, kind).map(|_| ())
}

pub fn stop(kind: Kind, unit: &str) -> Result<(), DbusError> {
    let mut message = dbus_message!("StopUnit")?;
    message.append_items(&[unit.into(), "fail".into()]);
    dbus_connect!(message, kind).map(|_| ())
}