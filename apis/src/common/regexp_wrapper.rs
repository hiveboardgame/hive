use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use web_sys::js_sys::RegExp;

pub struct RegExpWrapper {
    init: Arc<Mutex<dyn Fn() -> RegExp + Send + 'static>>,
}

impl RegExpWrapper {
    pub fn new<F>(init: F) -> Self
    where
        F: Fn() -> RegExp + Send + 'static,
    {
        Self {
            init: Arc::new(Mutex::new(init)),
        }
    }

    pub fn get(&self) -> RegExp {
        self.init.lock().unwrap()()
    }
}

lazy_static! {
    pub static ref NANOID_REGEX: RegExpWrapper =
        RegExpWrapper::new(|| RegExp::new("/game/(.*)", ""));
}
