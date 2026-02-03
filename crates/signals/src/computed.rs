use crate::{
    NodeKey,
    runtime::REACTIVE_SYSTEM,
    types::{Location, NodeInner, caller},
};
use std::{marker::PhantomData, ops::Deref};

pub struct Computed<T> {
    node: crate::types::NodeKey,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Clone for Computed<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            _marker: self._marker.clone(),
        }
    }
}

impl<T> Copy for Computed<T> {}

impl<T: 'static> Computed<T> {
    pub fn new<F>(getter: F, caller: Location) -> Self
    where
        F: Fn(Option<T>) -> T + 'static,
    {
        let node = REACTIVE_SYSTEM.with(|ctx| ctx.computed_new(getter, caller));
        Self {
            node,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn track(&self) {
        REACTIVE_SYSTEM.with(|ctx| {
            ctx.computed_track(self.node);
        });
    }

    pub fn read(&self) -> ComputedRef<'_, T> {
        self.track();
        ComputedRef::new(self.node)
    }

    pub fn peek(&self) -> ComputedRef<'_, T> {
        ComputedRef::new(self.node)
    }
}

impl<T: 'static + Clone> Computed<T> {
    pub fn get(&self) -> T {
        REACTIVE_SYSTEM.with(|ctx| ctx.computed_get(self.node))
    }
}

impl<T: PartialEq + 'static> Computed<T> {
    pub fn memo<F>(getter: F, caller: Location) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let node = REACTIVE_SYSTEM.with(move |ctx| ctx.computed_memo(getter, caller));
        Self {
            node,
            _marker: std::marker::PhantomData,
        }
    }
}

#[track_caller]
pub fn memo<T, F>(getter: F) -> Computed<T>
where
    T: PartialEq + 'static,
    F: Fn() -> T + 'static,
{
    Computed::memo(getter, caller())
}

#[track_caller]
pub fn computed<T, F>(getter: F) -> Computed<T>
where
    T: 'static,
    F: Fn(Option<T>) -> T + 'static,
{
    Computed::new(getter, caller())
}

pub struct ComputedRef<'a, T> {
    node: NodeKey,
    _marker: PhantomData<&'a T>,
}

impl<T> ComputedRef<'_, T> {
    pub fn new(node: NodeKey) -> Self {
        Self {
            node,
            _marker: PhantomData,
        }
    }
}

impl<T> Deref for ComputedRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let value = REACTIVE_SYSTEM.with(|ctx| {
            if let NodeInner::Computed(inner) = &ctx.inner.borrow().nodes[self.node].inner {
                unsafe { &*(inner.borrow().as_any() as *const dyn std::any::Any as *const T) }
            } else {
                panic!("Node is not a Computed");
            }
        });
        unsafe { &*(value as *const T) }
    }
}
