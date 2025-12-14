use std::sync::atomic::{AtomicBool, Ordering};

pub static DEBUG: AtomicBool = AtomicBool::new(false);

pub fn set_debug(enable: bool) {
    DEBUG.store(enable, Ordering::Relaxed);
}

pub fn is_debug() -> bool {
    DEBUG.load(Ordering::Relaxed)
}
