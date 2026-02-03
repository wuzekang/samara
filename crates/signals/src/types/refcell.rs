use serde::{Serializer, ser::SerializeStruct};
#[cfg(debug_assertions)]
use std::cell::RefCell as StdRefCell;
use std::cell::{Cell, UnsafeCell};
use std::ops::{Deref, DerefMut};

/// A clone of the standard library's `RefCell` type.
pub struct RefCell<T: ?Sized> {
    borrow: BorrowFlag,
    value: UnsafeCell<T>,
}

#[cfg(debug_assertions)]
pub type Location = &'static std::panic::Location<'static>;

#[cfg(debug_assertions)]
#[track_caller]
pub fn caller() -> Location {
    std::panic::Location::caller()
}

#[cfg(debug_assertions)]
pub fn serialize_location<S>(location: &Location, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = serializer.serialize_struct("Location", 3)?;
    state.serialize_field("file", location.file())?;
    state.serialize_field("line", &location.line())?;
    state.serialize_field("col", &location.column())?;
    state.end()
}

#[cfg(not(debug_assertions))]
pub type Location = ();

#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn caller() -> () {}

#[cfg(not(debug_assertions))]
pub fn serialize_location<S>(_: &Location, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeTuple;
    serializer.serialize_tuple(0)?.end()
}

/// An enumeration of values returned from the `state` method on a `RefCell<T>`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BorrowState {
    /// The cell is currently being read, there is at least one active `borrow`.
    Reading,
    /// The cell is currently being written to, there is an active `borrow_mut`.
    Writing,
    /// There are no outstanding borrows on this cell.
    Unused,
}

// Values [1, MAX-1] represent the number of `Ref` active
// (will not outgrow its range since `usize` is the size of the address space)
struct BorrowFlag {
    flag: Cell<usize>,

    #[cfg(debug_assertions)]
    locations: StdRefCell<Vec<Location>>,
}

const UNUSED: usize = 0;
const WRITING: usize = !0;

impl<T> RefCell<T> {
    /// Creates a new `RefCell` containing `value`.
    pub fn new(value: T) -> RefCell<T> {
        RefCell {
            borrow: BorrowFlag::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes the `RefCell`, returning the wrapped value.
    pub fn into_inner(self) -> T {
        debug_assert!(self.borrow.flag.get() == UNUSED);
        unsafe { self.value.into_inner() }
    }
}

impl<T: ?Sized> RefCell<T> {
    /// Immutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple
    /// immutable borrows can be taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    #[cfg_attr(debug_assertions, inline(never))]
    #[track_caller]
    pub fn borrow<'a>(&'a self) -> Ref<'a, T> {
        match BorrowRef::new(&self.borrow) {
            Some(b) => Ref {
                _value: unsafe { &*self.value.get() },
                _borrow: b,
            },
            None => self.panic("mutably borrowed"),
        }
    }

    /// Mutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `RefMut` exits scope. The value
    /// cannot be borrowed while this borrow is active.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently borrowed.
    #[cfg_attr(debug_assertions, inline(never))]
    #[track_caller]
    pub fn borrow_mut<'a>(&'a self) -> RefMut<'a, T> {
        match BorrowRefMut::new(&self.borrow) {
            Some(b) => RefMut {
                _value: unsafe { &mut *self.value.get() },
                _borrow: b,
            },
            None => self.panic("borrowed"),
        }
    }

    #[cfg(not(debug_assertions))]
    fn panic(&self, msg: &str) -> ! {
        panic!("RefCell<T> already {}", msg)
    }

    #[cfg(debug_assertions)]
    #[allow(unused_must_use)]
    fn panic(&self, msg: &str) -> ! {
        let mut msg = format!("RefCell<T> already {}", msg);
        let locations = self.borrow.locations.borrow();
        if locations.len() > 0 {
            msg.push_str("\ncurrent active borrows: \n");
            for b in locations.iter() {
                msg.push_str(&format!(
                    "-------------------------\n{}:{}:{}\n",
                    b.file(),
                    b.line(),
                    b.column()
                ));
            }
            msg.push_str("\n\n");
        }
        panic!("{}", msg)
    }
}

#[cfg(not(debug_assertions))]
impl BorrowFlag {
    #[inline]
    fn new() -> BorrowFlag {
        BorrowFlag {
            flag: Cell::new(UNUSED),
        }
    }

    #[inline]
    fn push(&self, _caller: Location) {}

    #[inline]
    fn pop(&self) {}
}

#[cfg(debug_assertions)]
impl BorrowFlag {
    fn new() -> BorrowFlag {
        BorrowFlag {
            flag: Cell::new(UNUSED),
            locations: StdRefCell::new(Vec::new()),
        }
    }

    fn push(&self, caller: Location) {
        self.locations.borrow_mut().push(caller);
    }

    fn pop(&self) {
        self.locations.borrow_mut().pop();
    }
}

unsafe impl<T: ?Sized> Send for RefCell<T> where T: Send {}

impl<T: Clone> Clone for RefCell<T> {
    #[inline]
    fn clone(&self) -> RefCell<T> {
        RefCell::new(self.borrow().clone())
    }
}

impl<T: Default> Default for RefCell<T> {
    #[inline]
    fn default() -> RefCell<T> {
        RefCell::new(Default::default())
    }
}

impl<T: ?Sized + PartialEq> PartialEq for RefCell<T> {
    #[inline]
    fn eq(&self, other: &RefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}

impl<T: ?Sized + Eq> Eq for RefCell<T> {}

struct BorrowRef<'b> {
    borrow: &'b BorrowFlag,
}

impl<'b> BorrowRef<'b> {
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline)]
    #[track_caller]
    fn new(borrow: &'b BorrowFlag) -> Option<BorrowRef<'b>> {
        let flag = borrow.flag.get();
        if flag == WRITING {
            return None;
        }
        borrow.flag.set(flag + 1);

        borrow.push(caller());
        Some(BorrowRef { borrow: borrow })
    }
}

impl<'b> Drop for BorrowRef<'b> {
    #[inline]
    fn drop(&mut self) {
        let flag = self.borrow.flag.get();
        debug_assert!(flag != WRITING && flag != UNUSED);
        self.borrow.flag.set(flag - 1);
        self.borrow.pop();
    }
}

/// Wraps a borrowed reference to a value in a `RefCell` box.
/// A wrapper type for an immutably borrowed value from a `RefCell<T>`.
///
/// See the [module-level documentation](index.html) for more.

pub struct Ref<'b, T: ?Sized + 'b> {
    // FIXME #12808: strange name to try to avoid interfering with
    // field accesses of the contained type via Deref
    _value: &'b T,
    _borrow: BorrowRef<'b>,
}

impl<'b, T: ?Sized> Deref for Ref<'b, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self._value
    }
}

struct BorrowRefMut<'b> {
    borrow: &'b BorrowFlag,
}

impl<'b> BorrowRefMut<'b> {
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline)]
    #[track_caller]
    fn new(borrow: &'b BorrowFlag) -> Option<BorrowRefMut<'b>> {
        if borrow.flag.get() != UNUSED {
            return None;
        }
        borrow.flag.set(WRITING);
        borrow.push(caller());
        Some(BorrowRefMut { borrow: borrow })
    }
}

impl<'b> Drop for BorrowRefMut<'b> {
    #[inline]
    fn drop(&mut self) {
        debug_assert!(self.borrow.flag.get() == WRITING);
        self.borrow.flag.set(UNUSED);
        self.borrow.pop();
    }
}

/// A wrapper type for a mutably borrowed value from a `RefCell<T>`.
pub struct RefMut<'b, T: ?Sized + 'b> {
    // FIXME #12808: strange name to try to avoid interfering with
    // field accesses of the contained type via Deref
    _value: &'b mut T,
    _borrow: BorrowRefMut<'b>,
}

impl<'b, T: ?Sized> Deref for RefMut<'b, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self._value
    }
}

impl<'b, T: ?Sized> DerefMut for RefMut<'b, T> {
    fn deref_mut(&mut self) -> &mut T {
        self._value
    }
}

pub struct UnsafeRefCell<T: ?Sized> {
    value: UnsafeCell<T>,
}

impl<T> UnsafeRefCell<T> {
    pub fn new(value: T) -> UnsafeRefCell<T> {
        UnsafeRefCell {
            value: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> UnsafeRefCell<T> {
    #[inline(always)]
    pub fn borrow<'a>(&'a self) -> &'a T {
        unsafe { &*self.value.get() }
    }

    #[inline(always)]
    pub fn borrow_mut<'a>(&'a self) -> &'a mut T {
        unsafe { &mut *self.value.get() }
    }
}

pub struct UnsafeBox<T: ?Sized> {
    value: *mut T,
}

impl<T> UnsafeBox<T> {
    pub fn new(value: T) -> UnsafeBox<T> {
        UnsafeBox {
            value: Box::leak(Box::new(value)),
        }
    }
}

impl<T: ?Sized> UnsafeBox<T> {
    #[inline(always)]
    pub fn borrow<'a>(&'a self) -> &'a T {
        unsafe { &*self.value }
    }

    #[inline(always)]
    pub fn borrow_mut<'a>(&'a self) -> &'a mut T {
        unsafe { &mut *self.value }
    }
}

impl<T: Default> Default for UnsafeBox<T> {
    fn default() -> Self {
        UnsafeBox {
            value: Box::leak(Box::new(T::default())),
        }
    }
}

impl<T> Clone for UnsafeBox<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for UnsafeBox<T> {}
