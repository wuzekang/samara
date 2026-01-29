use std::ops::{BitAnd, BitAndAssign, BitOr, Not};

/// Reactive node flags stored as a bitset for efficient operations.
///
/// Bit layout:
/// - Bit 0: MUTABLE - Node can be modified (signals)
/// - Bit 1: WATCHING - Node is an active effect/computed
/// - Bit 2: RECURSED_CHECK - Node is in recursion check phase
/// - Bit 3: RECURSED - Node has been visited during propagation
/// - Bit 4: DIRTY - Node needs recomputation
/// - Bit 5: PENDING - Node is queued for update
/// - Bits 6-7: Reserved for future use
#[derive(Clone, Copy, Debug)]
pub struct ReactiveFlags(pub u8);

impl ReactiveFlags {
    pub const NONE: Self = Self(0b0000_0000);
    pub const MUTABLE: Self = Self(0b0000_0001);
    pub const WATCHING: Self = Self(0b0000_0010);
    pub const RECURSED_CHECK: Self = Self(0b0000_0100);
    pub const RECURSED: Self = Self(0b0000_1000);
    pub const DIRTY: Self = Self(0b0001_0000);
    pub const PENDING: Self = Self(0b0010_0000);

    #[inline]
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    #[inline]
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline]
    pub fn intersects(&self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl BitOr for ReactiveFlags {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for ReactiveFlags {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for ReactiveFlags {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for ReactiveFlags {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl PartialEq for ReactiveFlags {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
