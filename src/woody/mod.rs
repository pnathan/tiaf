use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub enum Level {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
}

// https://doc.rust-lang.org/rust-by-example/hello/print/print_display.html
impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Level::Error => write!(f, "Error"),
            Level::Warn => write!(f, "Warn"),
            Level::Info => write!(f, "Info"),
            Level::Debug => write!(f, "Debug"),
        }
    }
}

impl Level {
    /// from_string converts s to a given Level. Upper or Lowercase are normalized.
    /// Valid values := error|warn|info|debug
    pub fn from_string(s: &str) -> Result<Level, String> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Level::Error),
            "warn" => Ok(Level::Warn),
            "info" => Ok(Level::Info),
            "debug" => Ok(Level::Debug),
            _ => Err(format!("{s} is not a valid level")),
        }
    }

    pub fn from_u8(x: &u8) -> Result<Level, String> {
        match x {
            0 => Ok(Level::Error),
            1 => Ok(Level::Warn),
            2 => Ok(Level::Info),
            3 => Ok(Level::Debug),
            _ => Err(format!("{x} is not a valid level")),
        }
    }
}

#[macro_export]
macro_rules! notes {
    // Do something interesting for a given pair of arguments
    ($a:expr, $b:expr) => {
        vec![Attributes::KV($a, $b)]
    };

    // Recursively traverse the arguments
    ($a:expr, $b:expr, $($rest:expr),+) => {
        {
            let mut v = vec![Attributes::KV($a, $b)];
            v.append(&mut notes!($($rest),*));
            v
        }
    };
    () => {};
}

/// Attributes is a list of key/value pairs that can be passed to the logger
pub enum Attributes<T, U>
where
    T: Display,
    U: Display,
{
    S(String),
    KV(T, U),
    Int(T, i64),
    String(T, String),
    Bool(T, bool),
    Float(T, f64),
}

/// Logger is a simple logger that can be used to log messages to stdout
#[derive(Clone)]
pub struct Logger {
    level: Level,
}

static LOGGER: OnceCell<Mutex<Logger>> = OnceCell::new();

// Returns an instance of our friend
pub fn new(level: Level) -> &'static Mutex<Logger> {
    LOGGER.get_or_init(|| Mutex::new(Logger { level }))
}

fn format<T: Display, U: Display>(attrs: Vec<Attributes<T, U>>) -> String {
    let mut pass_on: Vec<String> = Vec::new();
    for attr in attrs {
        match attr {
            Attributes::S(s) => pass_on.push(s),
            Attributes::KV(k, v) => pass_on.push(format!("{k}={v}")),
            Attributes::Int(k, v) => pass_on.push(format!("{k}={v}")),
            Attributes::String(k, v) => pass_on.push(format!("{k}={v}")),
            Attributes::Bool(k, v) => pass_on.push(format!("{k}={v}")),
            Attributes::Float(k, v) => pass_on.push(format!("{k}={v}")),
        }
    }
    pass_on.join(", ")
}

impl Logger {
    pub fn error<T: Display, U: Display>(&self, attrs: Vec<Attributes<T, U>>) {
        self.log(Level::Error, attrs);
    }
    pub fn warn<T: Display, U: Display>(&self, attrs: Vec<Attributes<T, U>>) {
        self.log(Level::Warn, attrs);
    }
    pub fn info<T: Display, U: Display>(&self, attrs: Vec<Attributes<T, U>>) {
        self.log(Level::Info, attrs);
    }
    pub fn debug<T: Display, U: Display>(&self, attrs: Vec<Attributes<T, U>>) {
        self.log(Level::Debug, attrs);
    }
    pub fn log<T: Display, U: Display>(&self, level: Level, attrs: Vec<Attributes<T, U>>) {
        if level <= self.level {
            let msg = format(attrs);
            println!("{level}, {msg}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logline() {
        let logger = new(Level::Info).lock().unwrap();
        logger.error(notes!("foo", "bar"));
        logger.warn(notes!("foo", "bar"));
        logger.info(notes!("foo", "bar"));
        logger.debug(notes!("foo", "bar"));
    }
}
