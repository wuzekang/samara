use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut};

use crate::runtime::REACTIVE_SYSTEM;
use crate::types::NodeKey;

pub struct Signal<T> {
    node: NodeKey,
    _marker: PhantomData<T>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Signal<T> {}

impl AddAssign<i32> for Signal<i32> {
    fn add_assign(&mut self, rhs: i32) {
        self.update(|value| *value += rhs);
    }
}

impl<T: 'static + Clone> Signal<T> {
    pub fn get(&self) -> T {
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal_get::<T>(self.node)
        })
    }
}

pub struct SignalWriteGuard<'a, T> {
    node: NodeKey,
    _marker: PhantomData<&'a mut T>,
}

impl<T> SignalWriteGuard<'_, T> {
    pub fn new(node: NodeKey) -> Self {
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal_borrow_write_check(node);
        });
        Self {
            node,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for SignalWriteGuard<'_, T> {
    fn drop(&mut self) {
        REACTIVE_SYSTEM.with(move |ctx| unsafe {
            let ctx = &mut *ctx.get();
            // Only release if node still exists
            if !ctx.nodes.contains_key(self.node) {
                return;
            }
            // Release borrow first
            ctx.signal_release_write(self.node);
            // Then notify subscribers
            ctx.signal_notify(self.node);
        });
    }
}

impl<T> Deref for SignalWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Check validity on every deref
        let value = REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal(self.node).value
        });
        unsafe { &*(value as *const T) }
    }
}

impl<T> DerefMut for SignalWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Check validity on every deref
        let value = REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal(self.node).value
        });
        unsafe { &mut *(value as *mut T) }
    }
}

pub struct SignalReadGuard<'a, T> {
    node: NodeKey,
    _marker: PhantomData<&'a T>,
}

impl<T> SignalReadGuard<'_, T> {
    pub fn new(node: NodeKey) -> Self {
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();

            // Check borrow state
            ctx.signal_borrow_read_check(node);

            // Track dependencies
            ctx.signal_track(node);
        });
        Self {
            node,
            _marker: PhantomData,
        }
    }
}

impl<T> Drop for SignalReadGuard<'_, T> {
    fn drop(&mut self) {
        REACTIVE_SYSTEM.with(move |ctx| unsafe {
            let ctx = &mut *ctx.get();
            // Only release if node still exists
            if ctx.nodes.contains_key(self.node) {
                ctx.signal_release_read(self.node);
            }
        });
    }
}

impl<T> Deref for SignalReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let value = REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal(self.node).value
        });
        unsafe { &*(value as *const T) }
    }
}

impl<T: 'static> Signal<T> {
    pub fn new(initial: T) -> Self {
        let node = REACTIVE_SYSTEM.with(move |ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal_new(initial)
        });
        Self {
            node,
            _marker: PhantomData,
        }
    }

    pub fn set(&self, new_value: T) {
        REACTIVE_SYSTEM.with(move |ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal_set::<T>(self.node, new_value)
        });
    }

    pub fn peek(&self) -> SignalReadGuard<'_, T> {
        let node = self.node;
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            // Check borrow but don't track dependencies
            ctx.signal_borrow_read_check(node);
        });
        SignalReadGuard {
            node,
            _marker: PhantomData,
        }
    }

    pub fn read(&self) -> SignalReadGuard<'_, T> {
        SignalReadGuard::new(self.node)
    }

    pub fn write(&self) -> SignalWriteGuard<'_, T> {
        SignalWriteGuard::new(self.node)
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.signal_update(self.node, f);
        });
    }
}

pub fn signal<T: 'static>(initial: T) -> Signal<T> {
    Signal::new(initial)
}
