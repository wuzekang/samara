use crate::runtime::REACTIVE_SYSTEM;
use crate::types::{Location, NodeKey, caller};

#[derive(Clone, Copy)]
pub struct Scope {
    node: NodeKey,
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope").field("node", &self.node).finish()
    }
}

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl Eq for Scope {}

impl PartialOrd for Scope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Scope {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.node.cmp(&other.node)
    }
}

impl std::hash::Hash for Scope {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

impl Scope {
    pub fn new(node: NodeKey) -> Self {
        Self { node }
    }

    /// The raw node key identifying this scope.
    pub fn key(&self) -> NodeKey {
        self.node
    }

    pub fn run<F: FnOnce() + 'static>(f: F, caller: Location) -> Self {
        let scope = REACTIVE_SYSTEM.with(move |ctx| ctx.new_scope(f, caller));
        Self { node: scope }
    }

    pub fn dispose(&self) {
        REACTIVE_SYSTEM.with(|ctx| {
            ctx.dispose_scope(self.node);
        });
    }
}

pub fn cleanup() {
    REACTIVE_SYSTEM.with(|ctx| {
        ctx.cleanup();
    })
}

/// Returns the scope current at this moment (the innermost scope or effect
/// being executed). Outside any reactive execution this is the root scope.
pub fn current_scope() -> Scope {
    REACTIVE_SYSTEM.with(|ctx| Scope::new(ctx.current_scope()))
}

/// The root scope every other scope descends from.
pub fn root_scope() -> Scope {
    REACTIVE_SYSTEM.with(|ctx| Scope::new(ctx.root()))
}

/// Creates a new scope and executes a function within it.
///
/// The scope will be the parent for any reactive primitives (signals, computed, effects)
/// created during the function execution. The returned `Scope` can be used to manually
/// cleanup all resources created within this scope.
///
/// # Example
/// ```rust
/// # use samara_signals::*;
/// let scope = scope(|| {
///     let s = signal(42);
///     // s will be automatically cleaned up when scope.cleanup() is called
/// });
/// scope.dispose();
/// ```
#[track_caller]
pub fn scope<F: FnOnce() + 'static>(f: F) -> Scope {
    Scope::run(f, caller())
}

/// Creates a closure that executes a function within a new child scope.
///
/// The parent scope is captured when this function is called, not when the
/// returned closure is executed. This ensures the child scope is always
/// created under the correct parent even if current_scope changes.
///
/// Unlike `scope`, which executes immediately, this returns a reusable closure
/// that creates a new child scope each time it's called.
///
/// # Type Parameters
/// - `T`: Input type for the function
/// - `U`: Return type of the function
///
/// # Example
/// ```rust
/// # use samara_signals::*;
/// let scoped_fn = scoped(|x: i32| x + 1);
/// let (result, scope) = scoped_fn(5);
/// assert_eq!(result, 6);
/// scope.dispose(); // Manually cleanup the child scope
/// ```
#[track_caller]
pub fn scoped<T, U>(f: impl Fn(T) -> U + 'static) -> impl Fn(T) -> (U, Scope)
where
    T: 'static,
{
    let caller = caller();
    // CAPTURE the current scope at closure creation time
    let parent_scope = REACTIVE_SYSTEM.with(|ctx| ctx.current_scope());

    move |t| {
        REACTIVE_SYSTEM.with(|ctx| {
            // Create child scope node with the CAPTURED parent
            let scope_node = ctx.new_child_scope(parent_scope, caller);

            // Set as current scope
            let prev_scope = ctx.current_scope();
            ctx.set_current_scope(scope_node);

            // Execute function
            let result = f(t);

            // Restore previous scope
            ctx.set_current_scope(prev_scope);

            (result, Scope::new(scope_node))
        })
    }
}
