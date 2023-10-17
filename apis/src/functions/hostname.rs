use std::fmt;
use web_sys;

pub struct Address {
    pub hostname: String,
    pub port: Option<String>,
}

impl Address {
    pub fn new(hostname: String, port: Option<String>) -> Address {
        Address { hostname, port }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.port.is_some() {
            write!(f, "{}:{}", self.hostname, self.port.as_ref().unwrap())
        } else {
            write!(f, "{}", self.hostname)
        }
    }
}

pub fn hostname_and_port() -> Address {
    let window = web_sys::window().expect("no global `window` exists");
    let location = window.location();
    let hostname = location
        .hostname()
        .expect("location should have a hostname");
    let port = location.port().ok().filter(|s| !s.is_empty());
    Address::new(hostname, port)
}
