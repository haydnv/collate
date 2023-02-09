//! Defines a [`Collate`] trait to standardize collation methods across data types. The provided
//! [`Collator`] struct can be used to collate a collection of slices of type `T` where `T: Ord`.
//!
//! [`Collate`] is useful for implementing a B-Tree, or to handle cases where a collator type is
//! more efficient than calling `Ord::cmp` repeatedly, for example when collating localized strings
//! using `rust_icu_ucol`. It's also useful to handle types like complex numbers which do not
//! necessarily have a natural ordering.

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::Range;
use std::sync::Arc;

#[cfg(feature = "complex")]
pub use complex::*;

/// Defines methods to collate a collection of slices of type `Value`, given a comparator.
pub trait Collate {
    type Value;

    /// Define the relative ordering of `Self::Value`.
    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering;
}

/// A generic collator for any type `T: Ord`.
#[derive(Default, Clone)]
pub struct Collator<T> {
    phantom: PhantomData<T>,
}

impl<T: Ord> Collate for Collator<T> {
    type Value = T;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        left.cmp(right)
    }
}

/// An [`Overlap`] is the result of a comparison between two ranges,
/// the equivalent of [`Ordering`] for hierarchical data.
#[derive(Debug, Eq, PartialEq, Copy, Clone, PartialOrd)]
pub enum Overlap {
    /// A lack of overlap where the compared range is entirely less than another
    Less,

    /// A lack of overlap where the compared range is entirely greater than another
    Greater,

    /// An overlap where the compared range is identical to another
    Equal,

    /// An overlap where the compared range is narrower than another
    Narrow,

    /// An overlap where the compared range is wider than another on both sides
    Wide,

    /// An overlap where the compared range is wider than another with a lesser start and end point
    WideLess,

    /// An overlap where the compared range is wider than another with a greater start and end point
    WideGreater,
}

/// Range comparison methods
pub trait Overlaps<T> {
    /// Check whether `other` lies entirely within `self`.
    #[inline]
    fn contains(&self, other: &T) -> bool {
        match self.overlaps(other) {
            Overlap::Wide | Overlap::Equal => true,
            _ => false,
        }
    }

    /// Check whether `other` lies at least partially within `self`.
    #[inline]
    fn contains_partial(&self, other: &T) -> bool {
        match self.overlaps(other) {
            Overlap::Narrow | Overlap::Equal => true,
            Overlap::WideLess | Overlap::Wide | Overlap::WideGreater => true,
            _ => false,
        }
    }

    /// Check whether `self` overlaps `other`.
    ///
    /// Examples:
    /// ```
    /// use collate::{Overlap, Overlaps};
    /// assert_eq!((0..1).overlaps(&(2..5)), Overlap::Less);
    /// assert_eq!((0..1).overlaps(&(0..1)), Overlap::Equal);
    /// assert_eq!((2..3).overlaps(&(0..2)), Overlap::Greater);
    /// assert_eq!((3..5).overlaps(&(1..7)), Overlap::Narrow);
    /// assert_eq!((1..7).overlaps(&(3..5)), Overlap::Wide);
    /// assert_eq!((1..4).overlaps(&(3..5)), Overlap::WideLess);
    /// assert_eq!((3..5).overlaps(&(1..4)), Overlap::WideGreater);
    /// ```
    fn overlaps(&self, other: &T) -> Overlap;
}

impl<T: Overlaps<T>> Overlaps<T> for Arc<T> {
    fn overlaps(&self, other: &T) -> Overlap {
        (&**self).overlaps(&other)
    }
}

impl<T: Overlaps<T>> Overlaps<Arc<T>> for Arc<T> {
    fn overlaps(&self, other: &Arc<T>) -> Overlap {
        (&**self).overlaps(&**other)
    }
}

impl<Idx: PartialOrd<Idx>> Overlaps<Range<Idx>> for Range<Idx> {
    fn overlaps(&self, other: &Self) -> Overlap {
        assert!(self.end >= self.start);
        assert!(other.end >= other.start);

        if self.start >= other.end {
            Overlap::Greater
        } else if self.end <= other.start {
            Overlap::Less
        } else if self.start == other.start && self.end == other.end {
            Overlap::Equal
        } else if self.start <= other.start && self.end >= other.end {
            Overlap::Wide
        } else if self.start >= other.start && self.end <= other.end {
            Overlap::Narrow
        } else if self.end > other.end {
            Overlap::WideGreater
        } else if self.start < other.start {
            Overlap::WideLess
        } else {
            unreachable!()
        }
    }
}
