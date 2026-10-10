use crate::system::ReactiveSystemRef;
use crate::types::{Location, RefCell};
use crate::{NodeKey, system::ReactiveSystem};
use std::rc::Rc;

pub mod executor;

thread_local! {
    pub static REACTIVE_SYSTEM: ReactiveRuntime = ReactiveRuntime::new();
}

pub struct ReactiveRuntime {
    pub inner: ReactiveSystemRef<ReactiveSystem>,
}

impl ReactiveRuntime {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ReactiveSystem::new())),
        }
    }

    // Context methods
    #[inline]
    pub fn provide_context<T: 'static>(&self, value: T) {
        self.inner.borrow_mut().provide_context(value);
    }

    #[inline]
    pub fn use_context<T: 'static + Clone>(&self) -> Option<T> {
        self.inner.borrow().use_context()
    }

    #[inline]
    pub fn has_context<T: 'static>(&self) -> bool {
        self.inner.borrow().has_context::<T>()
    }

    #[inline]
    pub fn new_effect<F: FnMut() + 'static>(&self, effect: F, caller: Location) -> NodeKey {
        ReactiveSystem::new_effect(&self.inner, effect, caller)
    }

    #[inline]
    pub fn new_scope<F: FnOnce() + 'static>(&self, f: F, caller: Location) -> NodeKey {
        ReactiveSystem::new_scope(&self.inner, f, caller)
    }

    #[inline]
    pub fn new_child_scope(&self, parent: NodeKey, caller: Location) -> NodeKey {
        self.inner.borrow_mut().new_child_scope(parent, caller)
    }

    #[inline]
    pub fn trigger<F: Fn() + 'static>(&self, f: F, caller: Location) {
        ReactiveSystem::trigger(&self.inner, f, caller);
    }

    #[inline]
    pub fn set_active_sub(&self, sub: Option<NodeKey>) -> Option<NodeKey> {
        self.inner.borrow().set_active_sub(sub)
    }

    /// Whether the given node key still refers to a live node.
    #[inline]
    pub fn node_exists(&self, node: NodeKey) -> bool {
        self.inner.borrow().nodes.contains_key(node)
    }

    #[inline]
    pub fn restore_acative_sub(&self, sub: Option<NodeKey>) {
        self.inner.borrow().active_sub.set(sub);
    }

    #[inline]
    pub fn dispose_scope(&self, node: NodeKey) {
        ReactiveSystem::dispose_scope(&self.inner, node);
    }

    #[inline]
    pub fn cleanup(&self) {
        ReactiveSystem::cleanup(&self.inner);
    }

    #[inline]
    pub fn computed_memo<F, T>(&self, getter: F, caller: Location) -> NodeKey
    where
        F: Fn() -> T + 'static,
        T: PartialEq + 'static,
    {
        self.inner.borrow_mut().computed_memo(getter, caller)
    }

    #[inline]
    pub fn computed_new<F, T>(&self, getter: F, caller: Location) -> NodeKey
    where
        F: Fn(Option<T>) -> T + 'static,
        T: 'static,
    {
        self.inner.borrow_mut().computed_new(getter, caller)
    }

    pub fn computed_track(&self, node: NodeKey) {
        ReactiveSystem::computed_track(&self.inner, node);
    }

    /// Ensure the computed is evaluated and fresh without tracking it.
    #[inline]
    pub fn computed_peek(&self, node: NodeKey) {
        ReactiveSystem::computed_ensure_fresh(&self.inner, node);
    }

    #[inline]
    pub fn computed_get<T>(&self, node: NodeKey) -> T
    where
        T: Clone + 'static,
    {
        ReactiveSystem::computed_get(&self.inner, node)
    }

    #[inline]
    pub fn signal_new<T: 'static>(&self, initial: T, caller: Location) -> NodeKey {
        self.inner.borrow_mut().signal_new(initial, caller)
    }

    /// Shared, downcast-checked pointer to a signal's value (read guards).
    ///
    /// Contract: the pointer is only valid while the node is alive and its
    /// value is not swapped out (signal update/with); guard derefs re-fetch
    /// it on every access and must not hold it across scope disposal.
    #[inline]
    pub fn signal_value_ref<T: 'static>(&self, node: NodeKey) -> *const T {
        let mut sys = self.inner.borrow_mut();
        sys.signal(node)
            .value
            .downcast_ref::<T>()
            .map(|v| v as *const T)
            .expect("Signal type mismatch")
    }

    /// Unique, downcast-checked pointer to a signal's value (write guards).
    ///
    /// Same contract as `signal_value_ref`.
    #[inline]
    pub fn signal_value_mut<T: 'static>(&self, node: NodeKey) -> *mut T {
        let mut sys = self.inner.borrow_mut();
        sys.signal(node)
            .value
            .downcast_mut::<T>()
            .map(|v| v as *mut T)
            .expect("Signal type mismatch")
    }

    #[inline]
    pub fn signal_track(&self, node: NodeKey) {
        self.inner.borrow_mut().signal_track(node);
    }

    #[inline]
    pub fn signal_get<T: 'static + Clone>(&self, node: NodeKey) -> T {
        self.inner.borrow_mut().signal_get(node)
    }

    #[inline]
    pub fn signal_notify(&self, node: NodeKey) {
        ReactiveSystem::signal_notify(&self.inner, node);
    }

    #[inline]
    pub fn signal_set<T: 'static>(&self, node: NodeKey, value: T) {
        ReactiveSystem::signal_set(&self.inner, node, value);
    }

    /// Run f on a shared reference to the value, without tracking.
    ///
    /// The value is swapped out for a ZST placeholder so `f` runs with no
    /// system borrow held and may re-enter the reactive system freely.
    #[inline]
    pub fn signal_with<T: 'static, O>(&self, node: NodeKey, f: impl FnOnce(&T) -> O) -> O {
        let taken = {
            let mut sys = self.inner.borrow_mut();
            let signal = sys.signal(node);
            signal.borrow_read_check();
            std::mem::replace(&mut signal.value, Box::new(()))
        };

        let result = f(taken.downcast_ref::<T>().expect("Signal type mismatch"));

        {
            let mut sys = self.inner.borrow_mut();
            if sys.nodes.contains_key(node) {
                let signal = sys.signal(node);
                signal.value = taken;
                signal.release_read();
            }
            // else: scope disposed inside f; `taken` frees the value on drop.
        }
        result
    }

    #[inline]
    pub fn signal_update<T: 'static>(&self, node: NodeKey, f: impl FnOnce(&mut T) -> ()) {
        ReactiveSystem::signal_update(&self.inner, node, f);
    }

    #[inline]
    pub fn signal_borrow_read_check(&self, node: NodeKey) {
        self.inner.borrow_mut().signal_borrow_read_check(node);
    }

    #[inline]
    pub fn signal_borrow_write_check(&self, node: NodeKey) {
        self.inner.borrow_mut().signal_borrow_write_check(node);
    }

    #[inline]
    pub fn signal_release_read(&self, node: NodeKey) {
        self.inner.borrow_mut().signal_release_read(node);
    }

    #[inline]
    pub fn signal_release_write(&self, node: NodeKey) {
        self.inner.borrow_mut().signal_release_write(node);
    }

    #[inline]
    pub fn start_batch(&self) {
        self.inner.borrow_mut().start_batch();
    }

    #[inline]
    pub fn end_batch(&self) {
        ReactiveSystem::end_batch(&self.inner);
    }

    #[inline]
    pub fn count(&self) -> (usize, usize) {
        self.inner.borrow().count()
    }

    #[inline]
    // Field accessors for internal use
    pub fn current_scope(&self) -> NodeKey {
        self.inner.borrow().current_scope.get()
    }

    #[inline]
    pub fn active_sub(&self) -> Option<NodeKey> {
        self.inner.borrow().active_sub.get()
    }

    #[inline]
    pub fn set_current_scope(&self, scope: NodeKey) {
        self.inner.borrow().current_scope.set(scope);
    }

    #[inline]
    pub fn on_cleanup<F: FnOnce() + 'static>(&self, f: F) {
        let current = self.inner.borrow_mut().current_scope.get();
        if let Some(cleanups) = self.inner.borrow_mut().cleanups.get_mut(current) {
            cleanups.push(Box::new(f));
        } else {
            self.inner
                .borrow_mut()
                .cleanups
                .insert(current, vec![Box::new(f)]);
        }
    }
}

// Implement Serialize for ReactiveRuntime by serializing the inner ReactiveSystem
impl serde::Serialize for ReactiveRuntime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.borrow().serialize(serializer)
    }
}
