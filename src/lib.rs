//! Defines a [`Collate`] trait to standardize collation methods across data types. The provided
//! [`Collator`] struct can be used to collate a collection of items of type `T` where `T: Ord`.
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

/// A collator for type `Value`.
pub trait Collate: Eq {
    type Value;

    /// Return the collation of the `left` value relative to the `right` value.
    fn cmp(&self, left: &Self::Value, right: &Self::Value) -> Ordering;
}

/// A generic collator for any type `T: Ord`.
#[derive(Default, Clone)]
pub struct Collator<T> {
    phantom: PhantomData<T>,
}

impl<T> PartialEq for Collator<T> {
    fn eq(&self, _other: &Self) -> bool {
        // this collator has no configuration state, and therefore must be identical
        // to any other collator of the same type
        true
    }
}

impl<T> Eq for Collator<T> {}

impl<T: Ord> Collate for Collator<T> {
    type Value = T;

    fn cmp(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
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

impl Overlap {
    /// Reverses the [`Overlap`] (e.g. `Less` becomes `Greater`, `Narrow` becomes `Wide`, etc).
    pub fn reverse(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::Greater => Self::Less,
            Self::Equal => Self::Equal,
            Self::Narrow => Self::Wide,
            Self::Wide => Self::Narrow,
            Self::WideLess => Self::WideGreater,
            Self::WideGreater => Self::WideLess,
        }
    }
}

/// Range comparison methods
pub trait Overlaps<T, C: Collate> {
    /// Check whether `other` lies entirely within `self`.
    #[inline]
    fn contains(&self, other: &T, collator: &C) -> bool {
        match self.overlaps(other, collator) {
            Overlap::Wide | Overlap::Equal => true,
            _ => false,
        }
    }

    /// Check whether `other` lies at least partially within `self`.
    #[inline]
    fn contains_partial(&self, other: &T, collator: &C) -> bool {
        match self.overlaps(other, collator) {
            Overlap::Narrow | Overlap::Equal => true,
            Overlap::WideLess | Overlap::Wide | Overlap::WideGreater => true,
            _ => false,
        }
    }

    /// Check whether `self` overlaps `other`.
    ///
    /// Examples:
    /// ```
    /// use collate::{Collator, Overlap, Overlaps};
    /// let collator = Collator::default();
    /// assert_eq!((0..1).overlaps(&(2..5), &collator), Overlap::Less);
    /// assert_eq!((0..1).overlaps(&(0..1), &collator), Overlap::Equal);
    /// assert_eq!((2..3).overlaps(&(0..2), &collator), Overlap::Greater);
    /// assert_eq!((3..5).overlaps(&(1..7), &collator), Overlap::Narrow);
    /// assert_eq!((1..7).overlaps(&(3..5), &collator), Overlap::Wide);
    /// assert_eq!((1..4).overlaps(&(3..5), &collator), Overlap::WideLess);
    /// assert_eq!((3..5).overlaps(&(1..4), &collator), Overlap::WideGreater);
    /// ```
    fn overlaps(&self, other: &T, collator: &C) -> Overlap;
}

impl<T: Overlaps<T, C>, C: Collate> Overlaps<T, C> for Arc<T>
where
    T: Overlaps<T, C>,
{
    fn overlaps(&self, other: &T, collator: &C) -> Overlap {
        (&**self).overlaps(&other, collator)
    }
}

impl<T: Overlaps<T, C>, C: Collate> Overlaps<Arc<T>, C> for Arc<T>
where
    T: Overlaps<T, C>,
{
    fn overlaps(&self, other: &Arc<T>, collator: &C) -> Overlap {
        (&**self).overlaps(&**other, collator)
    }
}

impl<Idx, C: Collate<Value = Idx>> Overlaps<Range<Idx>, C> for Range<Idx> {
    fn overlaps(&self, other: &Self, collator: &C) -> Overlap {
        debug_assert_ne!(collator.cmp(&self.end, &self.start), Ordering::Less);
        debug_assert_ne!(collator.cmp(&other.end, &other.start), Ordering::Less);

        let start = collator.cmp(&self.start, &other.start);
        let end = collator.cmp(&self.end, &other.end);

        match (start, end) {
            (Ordering::Equal, Ordering::Equal) => Overlap::Equal,

            (Ordering::Greater, Ordering::Less) => Overlap::Narrow,
            (Ordering::Equal, Ordering::Less) => Overlap::Narrow,
            (Ordering::Greater, Ordering::Equal) => Overlap::Narrow,

            (Ordering::Less, Ordering::Greater) => Overlap::Wide,
            (Ordering::Less, Ordering::Equal) => Overlap::Wide,
            (Ordering::Equal, Ordering::Greater) => Overlap::Wide,

            (Ordering::Greater, Ordering::Greater) => match collator.cmp(&self.start, &other.end) {
                Ordering::Less => Overlap::WideGreater,
                Ordering::Greater | Ordering::Equal => Overlap::Greater,
            },

            (Ordering::Less, Ordering::Less) => match collator.cmp(&self.end, &other.start) {
                Ordering::Greater => Overlap::WideLess,
                Ordering::Less | Ordering::Equal => Overlap::Less,
            },
        }
    }
}
