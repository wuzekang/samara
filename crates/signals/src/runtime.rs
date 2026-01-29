use std::cell::UnsafeCell;

use crate::system::ReactiveSystem;

pub mod executor;

thread_local! {
    pub static REACTIVE_SYSTEM: UnsafeCell<ReactiveSystem> = UnsafeCell::new(ReactiveSystem::new());
}
