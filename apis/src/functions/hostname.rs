use leptos_use::use_window;
use std::fmt;

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
            write!(
                f,
                "{}:{}",
                self.hostname,
                self.port.as_ref().expect("Port is some")
            )
        } else {
            write!(f, "{}", self.hostname)
        }
    }
}

pub fn hostname_and_port() -> Address {
    let window = use_window();
    if window.is_some() {
        let location = window.as_ref().expect("Window is some").location();
        let hostname = location
            .hostname()
            .expect("location should have a hostname");
        let port = location.port().ok().filter(|s| !s.is_empty());
        return Address::new(hostname, port);
    }
    Address::new(String::new(), None)
}
