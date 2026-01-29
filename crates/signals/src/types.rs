use std::{any::Any, cell::Cell, fmt::Debug};

use ::slotmap::new_key_type;

mod slotmap;

new_key_type! {
    pub struct LinkKey;
    pub struct NodeKey;
}

/// Trait for type-erased derived node operations
pub trait ComputedOps {
    fn update(&mut self);
    fn dirty(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
}

/// Computed node without equality check (always dirty after update)
pub struct ComputedNodeInner<T> {
    value: Option<T>,
    getter: Box<dyn Fn() -> T + 'static>,
}

impl<T: 'static> ComputedNodeInner<T> {
    pub fn new(getter: Box<dyn Fn() -> T + 'static>) -> Self {
        Self {
            value: None,
            getter,
        }
    }
}

impl<T: 'static> ComputedOps for ComputedNodeInner<T> {
    #[inline]
    fn update(&mut self) {
        self.value = Some((self.getter)());
    }

    #[inline]
    fn dirty(&self) -> bool {
        // Always dirty after first update
        self.value.is_some()
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self.value.as_ref().unwrap()
    }
}

/// Memo node with equality check (only dirty if value changed)
pub struct MemoNodeInner<T: PartialEq> {
    prev: Option<T>,
    curr: Option<T>,
    getter: Box<dyn Fn() -> T + 'static>,
}

impl<T: PartialEq + 'static> MemoNodeInner<T> {
    pub fn new(getter: Box<dyn Fn() -> T + 'static>) -> Self {
        Self {
            prev: None,
            curr: None,
            getter,
        }
    }

    pub fn value(&self) -> &T {
        self.curr.as_ref().or(self.prev.as_ref()).unwrap()
    }
}

impl<T: PartialEq + 'static> ComputedOps for MemoNodeInner<T> {
    #[inline]
    fn update(&mut self) {
        let new_value = (self.getter)();
        match (&self.prev, &self.curr) {
            (None, None) | (None, Some(_)) => {
                self.prev = self.curr.take();
                self.curr = Some(new_value);
            }
            (Some(_), None) => {
                self.curr = Some(new_value);
            }
            (Some(_), Some(_)) => {
                std::mem::swap(&mut self.prev, &mut self.curr);
                self.curr = Some(new_value);
            }
        }
    }

    #[inline]
    fn dirty(&self) -> bool {
        match (&self.prev, &self.curr) {
            (Some(prev_val), Some(curr_val)) => prev_val != curr_val,
            _ => false,
        }
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self.value()
    }
}

/// Borrow state for runtime borrow checking (like RefCell)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BorrowState {
    Unused,         // No active borrows
    Reading(usize), // Number of active read guards
    Writing,        // Active write guard (exclusive)
}

pub struct SignalNode {
    pub value: *mut dyn Any,
    pub borrow_state: Cell<BorrowState>,
}

impl SignalNode {
    /// Check if a read borrow is allowed, panic if not
    #[inline]
    pub fn borrow_read_check(&self) {
        match self.borrow_state.get() {
            BorrowState::Unused => {
                self.borrow_state.set(BorrowState::Reading(1));
            }
            BorrowState::Reading(count) => {
                self.borrow_state.set(BorrowState::Reading(count + 1));
            }
            BorrowState::Writing => {
                panic!("Cannot borrow signal as readable while already borrowed as writable");
            }
        }
    }

    /// Check if a write borrow is allowed, panic if not
    #[inline]
    pub fn borrow_write_check(&self) {
        match self.borrow_state.get() {
            BorrowState::Unused => {
                self.borrow_state.set(BorrowState::Writing);
            }
            BorrowState::Reading(_) => {
                panic!("Cannot borrow signal as writable while already borrowed as readable");
            }
            BorrowState::Writing => {
                panic!("Cannot have multiple write guards to the same signal");
            }
        }
    }

    /// Release a read borrow
    #[inline]
    pub fn release_read(&self) {
        match self.borrow_state.get() {
            BorrowState::Reading(count) if count > 1 => {
                self.borrow_state.set(BorrowState::Reading(count - 1));
            }
            BorrowState::Reading(1) => {
                self.borrow_state.set(BorrowState::Unused);
            }
            _ => panic!("Invalid borrow state during read release"),
        }
    }

    /// Release a write borrow
    #[inline]
    pub fn release_write(&self) {
        match self.borrow_state.get() {
            BorrowState::Writing => {
                self.borrow_state.set(BorrowState::Unused);
            }
            _ => panic!("Invalid borrow state during write release"),
        }
    }
}

impl Drop for SignalNode {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.value);
        }
    }
}

pub struct EffectNode {
    pub effect: Box<dyn Fn()>,
}

pub enum NodeInner {
    Effect(EffectNode),
    Computed(Box<dyn ComputedOps>),
    Signal(SignalNode),
    None,
}

impl Debug for NodeInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Effect(_) => write!(f, "Effect"),
            Self::Computed(_) => write!(f, "Computed"),
            Self::Signal(_) => write!(f, "Signal"),
            Self::None => write!(f, "None"),
        }
    }
}

#[derive(Debug)]
pub struct ReactiveNode {
    pub inner: NodeInner,
    pub deps: Option<LinkKey>,
    pub deps_tail: Option<LinkKey>,
    pub subs: Option<LinkKey>,
    pub subs_tail: Option<LinkKey>,
    pub parent: Option<NodeKey>,
    pub child: Option<NodeKey>,
    pub next: Option<NodeKey>,
    pub prev: Option<NodeKey>,
    pub flags: ReactiveFlags,
}

impl ReactiveNode {
    /// Create a new reactive node with the given inner type, flags, and parent.
    pub(crate) fn new(inner: NodeInner, flags: ReactiveFlags, parent: Option<NodeKey>) -> Self {
        Self {
            inner,
            deps: None,
            deps_tail: None,
            subs: None,
            subs_tail: None,
            flags,
            parent,
            child: None,
            next: None,
            prev: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Link {
    pub version: usize,
    pub dep: NodeKey,
    pub sub: NodeKey,
    pub prev_sub: Option<LinkKey>,
    pub next_sub: Option<LinkKey>,
    pub prev_dep: Option<LinkKey>,
    pub next_dep: Option<LinkKey>,
}

pub use crate::flags::ReactiveFlags;
pub use crate::types::slotmap::UnsafeSlotMap;
