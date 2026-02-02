use crate::runtime::REACTIVE_SYSTEM;

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
    pub fn new<F>(getter: F, caller: &'static std::panic::Location<'static>) -> Self
    where
        F: Fn(Option<T>) -> T + 'static,
    {
        let node = REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.computed_new(getter, caller)
        });
        Self {
            node,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn read(&self) -> &T {
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.computed_read(self.node)
        })
    }
}

impl<T: 'static + Clone> Computed<T> {
    pub fn get(&self) -> T {
        REACTIVE_SYSTEM.with(|ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.computed_get(self.node)
        })
    }
}

impl<T: PartialEq + 'static> Computed<T> {
    pub fn memo<F>(getter: F, caller: &'static std::panic::Location<'static>) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let node = REACTIVE_SYSTEM.with(move |ctx| unsafe {
            let ctx = &mut *ctx.get();
            ctx.computed_memo(getter, caller)
        });
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
    Computed::memo(getter, std::panic::Location::caller())
}

#[track_caller]
pub fn computed<T, F>(getter: F) -> Computed<T>
where
    T: 'static,
    F: Fn(Option<T>) -> T + 'static,
{
    Computed::new(getter, std::panic::Location::caller())
}
