use crate::system::ReactiveSystemRef;
use crate::types::Location;
use crate::{NodeKey, system::ReactiveSystem};

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
            inner: ReactiveSystemRef::new(ReactiveSystem::new()),
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
        ReactiveSystem::new_effect(self.inner.clone(), effect, caller)
    }

    #[inline]
    pub fn new_scope<F: FnOnce() + 'static>(&self, f: F, caller: Location) -> NodeKey {
        ReactiveSystem::new_scope(self.inner.clone(), f, caller)
    }

    #[inline]
    pub fn new_child_scope(&self, parent: NodeKey, caller: Location) -> NodeKey {
        self.inner.borrow_mut().new_child_scope(parent, caller)
    }

    #[inline]
    pub fn trigger<F: Fn() + 'static>(&self, f: F, caller: Location) {
        ReactiveSystem::trigger(self.inner.clone(), f, caller);
    }

    #[inline]
    pub fn set_active_sub(&self, sub: Option<NodeKey>) -> Option<NodeKey> {
        self.inner.borrow().set_active_sub(sub)
    }

    #[inline]
    pub fn restore_acative_sub(&self, sub: Option<NodeKey>) {
        self.inner.borrow().active_sub.set(sub);
    }

    #[inline]
    pub fn dispose_scope(&self, node: NodeKey) {
        ReactiveSystem::dispose_scope(self.inner.clone(), node);
    }

    #[inline]
    pub fn cleanup(&self) {
        ReactiveSystem::cleanup(self.inner.clone());
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
        ReactiveSystem::computed_track(self.inner.clone(), node);
    }

    #[inline]
    pub fn computed_get<T>(&self, node: NodeKey) -> T
    where
        T: Clone + 'static,
    {
        ReactiveSystem::computed_get(self.inner.clone(), node)
    }

    #[inline]
    pub fn signal_new<T: 'static>(&self, initial: T, caller: Location) -> NodeKey {
        self.inner.borrow_mut().signal_new(initial, caller)
    }

    #[inline]
    pub fn signal_value(&self, node: NodeKey) -> *mut (dyn std::any::Any + 'static) {
        self.inner.borrow_mut().signal(node).value
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
        ReactiveSystem::signal_notify(self.inner.clone(), node);
    }

    #[inline]
    pub fn signal_set<T: 'static>(&self, node: NodeKey, value: T) {
        ReactiveSystem::signal_set(self.inner.clone(), node, value);
    }

    #[inline]
    pub fn signal_with<T: 'static, O>(&self, node: NodeKey, f: impl FnOnce(&T) -> O) -> O {
        let value = { self.inner.borrow_mut().signal(node).value };
        f(unsafe { &*(value as *const dyn std::any::Any as *const T) })
    }

    #[inline]
    pub fn signal_update<T: 'static>(&self, node: NodeKey, f: impl FnOnce(&mut T) -> ()) {
        ReactiveSystem::signal_update(self.inner.clone(), node, f);
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
        ReactiveSystem::end_batch(self.inner.clone());
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
