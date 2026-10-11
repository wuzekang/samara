use crate::runtime::REACTIVE_SYSTEM;
use crate::types::{Location, NodeKey, caller};

#[derive(Clone, Copy)]
pub struct Effect {
    node: NodeKey,
}

impl std::fmt::Debug for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Effect").field("node", &self.node).finish()
    }
}

impl Effect {
    pub fn new<F: FnMut() + 'static>(effect: F, caller: Location) -> Self {
        let node = REACTIVE_SYSTEM.with(move |ctx| ctx.new_effect(effect, caller));
        Self { node }
    }

    /// Like [`new`], but the effect re-runs until the graph settles when its
    /// body writes a signal it depends on (cascade to fixpoint) instead of
    /// deferring the invalidation.
    pub fn new_cascade<F: FnMut() + 'static>(effect: F, caller: Location) -> Self {
        let node = REACTIVE_SYSTEM.with(move |ctx| ctx.new_effect_cascade(effect, caller));
        Self { node }
    }
    pub fn dispose(&self) {
        REACTIVE_SYSTEM.with(|ctx| {
            ctx.dispose_scope(self.node);
        });
    }
}

#[track_caller]
pub fn effect<F: FnMut() + 'static>(effect: F) -> Effect {
    Effect::new(effect, caller())
}

/// Create an effect whose self-writes cascade: when its body writes a
/// signal it depends on, it re-runs until the graph settles.
#[track_caller]
pub fn effect_recursive<F: FnMut() + 'static>(effect: F) -> Effect {
    Effect::new_cascade(effect, caller())
}

#[track_caller]
pub fn trigger<F: Fn() + 'static>(f: F) {
    REACTIVE_SYSTEM.with(move |ctx| {
        ctx.trigger(f, caller());
    });
}

/// Runs `f` without dependency tracking: signal reads inside `f` do not
/// subscribe the current effect or scope.
///
/// The current scope is preserved, so reactive values created inside `f`
/// are still owned by the enclosing scope. Only tracking is suspended.
///
/// # Example
///
/// ```rust
/// # use samara_signals::*;
/// let src = signal(1);
/// let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
/// let log2 = log.clone();
/// let _e = effect(move || {
///     // Tracked read: effect re-runs when src changes
///     let tracked = src.get();
///     // Untracked read: changes to aux do NOT re-run the effect
///     untrack(|| {
///         log2.borrow_mut().push((tracked, src.get()));
///     });
/// });
/// assert_eq!(*log.borrow(), vec![(1, 1)]);
/// ```
pub fn untrack<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev_sub = REACTIVE_SYSTEM.with(|ctx| ctx.set_active_sub(None));
    let result = f();
    REACTIVE_SYSTEM.with(|ctx| ctx.restore_acative_sub(prev_sub));
    result
}

/// Returns the effect that is currently tracking signal reads, if any.
///
/// Inside an effect body this is that effect. Inside a plain [`scope`](crate::scope)
/// it is the scope node. Outside any reactive context it is `None`.
/// The returned handle can be used with [`Effect::enter_context`] to
/// re-establish tracking around foreign-driven execution (e.g. polling a
/// future on an external executor).
pub fn current_effect() -> Option<Effect> {
    REACTIVE_SYSTEM.with(|ctx| {
        let node = ctx.active_sub()?;
        if ctx.node_exists(node) {
            Some(Effect { node })
        } else {
            None
        }
    })
}

impl Effect {
    /// The raw node key identifying this effect.
    pub fn key(&self) -> NodeKey {
        self.node
    }

    /// Enters this effect's tracking context, returning a guard that
    /// restores the previous context on drop.
    ///
    /// While the guard is alive, signal reads subscribe to this effect and
    /// reactive values are scoped to it — the same context an effect body
    /// runs in. This is the integration point for executors that need to
    /// keep tracking alive across `await` points or callback boundaries.
    pub fn enter_context(&self) -> EffectContextGuard {
        EffectContextGuard::new(self.node)
    }
}

/// Restores the reactive tracking context when dropped.
///
/// Created by [`Effect::enter_context`].
pub struct EffectContextGuard {
    node: NodeKey,
    prev_scope: NodeKey,
    prev_sub: Option<NodeKey>,
}

impl EffectContextGuard {
    fn new(node: NodeKey) -> Self {
        let (prev_scope, prev_sub) = REACTIVE_SYSTEM.with(|ctx| {
            let prev_scope = ctx.current_scope();
            let prev_sub = ctx.set_active_sub(Some(node));
            ctx.set_current_scope(node);
            (prev_scope, prev_sub)
        });
        Self {
            node,
            prev_scope,
            prev_sub,
        }
    }
}

impl Drop for EffectContextGuard {
    fn drop(&mut self) {
        REACTIVE_SYSTEM.with(|ctx| {
            // The effect may have been disposed while its context was
            // entered; restoring into a removed node would corrupt the
            // current-scope chain, so fall back to the root.
            let scope = if ctx.node_exists(self.node) {
                self.prev_scope
            } else {
                ctx.root()
            };
            ctx.set_current_scope(scope);
            ctx.restore_acative_sub(self.prev_sub);
        });
    }
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
    REACTIVE_SYSTEM.with(|ctx| {
        ctx.on_cleanup(f);
    });
}

pub fn start_batch() {
    REACTIVE_SYSTEM.with(|ctx| {
        ctx.start_batch();
    });
}

pub fn end_batch() {
    REACTIVE_SYSTEM.with(|ctx| {
        ctx.end_batch();
    });
}

/// Returns the current counts of nodes and links in the reactive system.
///
/// Returns a tuple of `(nodes_count, links_count)`.
pub fn count() -> (usize, usize) {
    REACTIVE_SYSTEM.with(|ctx| ctx.count())
}

pub fn serialize() -> String {
    REACTIVE_SYSTEM.with(|ctx| serde_json::ser::to_string(ctx).unwrap())
}
