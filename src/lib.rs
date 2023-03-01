//! Defines a [`Collate`] trait to standardize collation methods across data types. The provided
//! [`Collator`] struct can be used to collate a collection of items of type `T` where `T: Ord`.
//!
//! [`Collate`] is useful for implementing a B-Tree, or to handle cases where a collator type is
//! more efficient than calling `Ord::cmp` repeatedly, for example when collating localized strings
//! using `rust_icu_ucol`. It's also useful to handle types like complex numbers which do not
//! necessarily have a natural ordering.

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

#[cfg(feature = "complex")]
pub use complex::*;

/// A collator for type `Value`.
pub trait Collate: Sized + Eq {
    type Value;

    /// Return the collation of the `left` value relative to the `right` value.
    fn cmp(&self, left: &Self::Value, right: &Self::Value) -> Ordering;

    /// Return `true` if the given `range` contains the given `value`.
    fn contains<R>(&self, range: &R, value: &Self::Value) -> bool
    where
        R: RangeBounds<Self::Value>,
    {
        let start = match range.start_bound() {
            Bound::Unbounded => Ordering::Less,
            Bound::Included(start) => self.cmp(start, value),
            Bound::Excluded(start) => match self.cmp(start, value) {
                Ordering::Less | Ordering::Equal => Ordering::Less,
                Ordering::Greater => Ordering::Greater,
            }
        };

        let end = match range.end_bound() {
            Bound::Unbounded => Ordering::Greater,
            Bound::Included(end) => self.cmp(end, value),
            Bound::Excluded(end) => match self.cmp(end, value) {
                Ordering::Less => Ordering::Less,
                Ordering::Greater | Ordering::Equal => Ordering::Greater,
            }
        };

        match (start, end) {
            (Ordering::Equal, _) => true,
            (_, Ordering::Equal) => true,
            (Ordering::Less, Ordering::Greater) => true,
            (Ordering::Less, Ordering::Less) => false,
            (Ordering::Greater, Ordering::Greater) => false,
            (Ordering::Greater, Ordering::Less) => panic!("bad range"),
        }
    }

    /// Return the [`Overlap`] of the `left` range w/r/t the `right` range.
    fn overlaps<L, R>(&self, left: &L, right: &R) -> Overlap
    where
        L: RangeBounds<Self::Value>,
        R: RangeBounds<Self::Value>,
    {
        let start = cmp_bound_start(self, left.start_bound(), right.start_bound());
        let end = cmp_bound_end(self, left.end_bound(), right.end_bound());

        match (start, end) {
            (Ordering::Equal, Ordering::Equal) => Overlap::Equal,

            (Ordering::Greater, Ordering::Less) => Overlap::Narrow,
            (Ordering::Greater, Ordering::Equal) => Overlap::Narrow,
            (Ordering::Equal, Ordering::Less) => Overlap::Narrow,

            (Ordering::Less, Ordering::Greater) => Overlap::Wide,
            (Ordering::Less, Ordering::Equal) => Overlap::WideLess,
            (Ordering::Equal, Ordering::Greater) => Overlap::WideGreater,

            (Ordering::Less, _) => {
                match cmp_bound_start(self, left.end_bound(), right.start_bound()) {
                    Ordering::Less => Overlap::Less,
                    Ordering::Greater | Ordering::Equal => Overlap::WideLess,
                }
            }

            (_, Ordering::Greater) => {
                match cmp_bound_end(self, left.start_bound(), right.end_bound()) {
                    Ordering::Less | Ordering::Equal => Overlap::WideGreater,
                    Ordering::Greater => Overlap::Greater,
                }
            }
        }
    }
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
    /// use collate::{Collate, Collator, Overlap};
    /// let collator = Collator::default();
    /// assert_eq!(collator.overlaps(&(..1), &(2..5)), Overlap::Less);
    /// assert_eq!(collator.overlaps(&(0..1), &(0..1)), Overlap::Equal);
    /// assert_eq!(collator.overlaps(&(2..3), &(..2)), Overlap::Greater);
    /// assert_eq!(collator.overlaps(&(3..5), &(1..7)), Overlap::Narrow);
    /// assert_eq!(collator.overlaps(&(1..), &(3..5)), Overlap::Wide);
    /// assert_eq!(collator.overlaps(&(1..4), &(3..)), Overlap::WideLess);
    /// assert_eq!(collator.overlaps(&(3..5), &(1..4)), Overlap::WideGreater);
    /// ```
    fn overlaps(&self, other: &T, collator: &C) -> Overlap;
}

#[inline]
fn cmp_bound_start<C>(collator: &C, left: Bound<&C::Value>, right: Bound<&C::Value>) -> Ordering
where
    C: Collate,
{
    match (left, right) {
        (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
        (Bound::Unbounded, _) => Ordering::Less,
        (_, Bound::Unbounded) => Ordering::Greater,
        (Bound::Included(this), Bound::Included(that)) => collator.cmp(this, that),
        (Bound::Excluded(this), Bound::Excluded(that)) => collator.cmp(this, that),
        (Bound::Included(this), Bound::Excluded(that)) => match collator.cmp(this, that) {
            Ordering::Less | Ordering::Equal => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
        },
        (Bound::Excluded(this), Bound::Included(that)) => match collator.cmp(this, that) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater | Ordering::Equal => Ordering::Greater,
        },
    }
}

#[inline]
fn cmp_bound_end<C>(collator: &C, left: Bound<&C::Value>, right: Bound<&C::Value>) -> Ordering
where
    C: Collate,
{
    match (left, right) {
        (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
        (Bound::Unbounded, _) => Ordering::Greater,
        (_, Bound::Unbounded) => Ordering::Less,
        (Bound::Included(this), Bound::Included(that)) => collator.cmp(this, that),
        (Bound::Excluded(this), Bound::Excluded(that)) => collator.cmp(this, that),
        (Bound::Included(this), Bound::Excluded(that)) => match collator.cmp(this, that) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater | Ordering::Equal => Ordering::Greater,
        },
        (Bound::Excluded(this), Bound::Included(that)) => match collator.cmp(this, that) {
            Ordering::Greater => Ordering::Greater,
            Ordering::Less | Ordering::Equal => Ordering::Less,
        },
    }
}
