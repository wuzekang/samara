use crate::runtime::REACTIVE_SYSTEM;
use crate::types::NodeKey;

#[derive(Clone, Copy)]
pub struct Effect {
    node: NodeKey,
}

impl Effect {
    pub fn new<F: Fn() + 'static>(effect: F) -> Self {
        let node = REACTIVE_SYSTEM.with(move |ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.new_effect(effect)
        });
        Self { node }
    }
    pub fn dispose(&self) {
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.dispose_scope(self.node);
        });
    }
}

pub fn effect<F: Fn() + 'static>(effect: F) -> Effect {
    Effect::new(effect)
}

pub fn trigger<F: Fn() + 'static>(f: F) {
    REACTIVE_SYSTEM.with(move |ctx| unsafe {
        let ctx = &mut *ctx.get();

        ctx.trigger(f);
    });
}

/// Register a cleanup callback to be called when the current scope is destroyed.
///
/// The cleanup function will be called in LIFO order (last registered, first called).
///
/// # Panics
///
/// Panics if called outside of any reactive scope (effect or scope).
///
/// # Example
///
/// ```rust
/// # use samara_signals::*;
/// let scope = scope(|| {
///     let id = 1;
///     on_cleanup(move || {
///         println!("Cleaning up {}", id);
///     });
/// });
/// scope.dispose(); // Prints: "Cleaning up 1"
/// ```
pub fn on_cleanup<F: FnOnce() + 'static>(f: F) {
    REACTIVE_SYSTEM.with(|ctx| unsafe {
        let ctx = &mut *ctx.get();
        let current = ctx.current_scope.get();
        if let Some(cleanups) = ctx.cleanups.get_mut(current) {
            cleanups.push(Box::new(f));
        } else {
            ctx.cleanups.insert(current, vec![Box::new(f)]);
        }
    });
}

pub fn start_batch() {
    REACTIVE_SYSTEM.with(|ctx| unsafe {
        let ctx = &mut *ctx.get();
        ctx.start_batch();
    });
}

pub fn end_batch() {
    REACTIVE_SYSTEM.with(|ctx| unsafe {
        let ctx = &mut *ctx.get();
        ctx.end_batch();
    });
}

/// Returns the current counts of nodes and links in the reactive system.
///
/// Returns a tuple of `(nodes_count, links_count)`.
pub fn count() -> (usize, usize) {
    REACTIVE_SYSTEM.with(|ctx| unsafe {
        let ctx = &mut *ctx.get();
        ctx.count()
    })
}
