//! Defines a [`Collate`] trait to standardize collation methods across data types. The provided
//! [`Collator`] struct can be used to collate a collection of items of type `T` where `T: Ord`.
//!
//! [`Collate`] is useful for implementing a B-Tree, or to handle cases where a collator type is
//! more efficient than calling `Ord::cmp` repeatedly, for example when collating localized strings
//! using `rust_icu_ucol`. It's also useful to handle types like complex numbers which do not
//! necessarily have a natural ordering.

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

#[cfg(feature = "complex")]
pub use complex::*;

/// A collator for type `Value`.
pub trait Collate: Sized + Eq {
    type Value;

    /// Return the collation of the `left` value relative to the `right` value.
    fn cmp(&self, left: &Self::Value, right: &Self::Value) -> Ordering;

    // /// Return `true` if the given `range` contains the given `value`.
    // fn contains<R>(&self, range: &R, value: &Self::Value) -> bool
    // where
    //     R: RangeBounds<Self::Value>,
    // {
    //     let start = match range.start_bound() {
    //         Bound::Unbounded => Ordering::Less,
    //         Bound::Included(start) => self.cmp(start, value),
    //         Bound::Excluded(start) => match self.cmp(start, value) {
    //             Ordering::Less | Ordering::Equal => Ordering::Less,
    //             Ordering::Greater => Ordering::Greater,
    //         }
    //     };
    //
    //     let end = match range.end_bound() {
    //         Bound::Unbounded => Ordering::Greater,
    //         Bound::Included(end) => self.cmp(end, value),
    //         Bound::Excluded(end) => match self.cmp(end, value) {
    //             Ordering::Less => Ordering::Less,
    //             Ordering::Greater | Ordering::Equal => Ordering::Greater,
    //         }
    //     };
    //
    //     match (start, end) {
    //         (Ordering::Equal, _) => true,
    //         (_, Ordering::Equal) => true,
    //         (Ordering::Less, Ordering::Greater) => true,
    //         (Ordering::Less, Ordering::Less) => false,
    //         (Ordering::Greater, Ordering::Greater) => false,
    //         (Ordering::Greater, Ordering::Less) => panic!("bad range"),
    //     }
    // }
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
    /// use collate::{Collate, Collator, Overlap, Overlaps};
    /// let collator = Collator::default();
    /// assert_eq!((..1).overlaps(&(2..5), &collator), Overlap::Less);
    /// assert_eq!((0..1).overlaps(&(0..1), &collator), Overlap::Equal);
    /// assert_eq!((1..4).overlaps(&(4..5), &collator), Overlap::Less);
    /// assert_eq!((4..5).overlaps(&(1..4), &collator), Overlap::Greater);
    /// assert_eq!((3..5).overlaps(&(1..7), &collator), Overlap::Narrow);
    /// assert_eq!((1..).overlaps(&(3..5), &collator), Overlap::Wide);
    /// assert_eq!((1..4).overlaps(&(3..), &collator), Overlap::WideLess);
    /// assert_eq!((3..5).overlaps(&(..4), &collator), Overlap::WideGreater);
    /// ```
    fn overlaps(&self, other: &T, collator: &C) -> Overlap;
}

impl Overlaps<(Bound<usize>, Bound<usize>), Collator<usize>> for (Bound<usize>, Bound<usize>) {
    fn overlaps(
        &self,
        other: &(Bound<usize>, Bound<usize>),
        collator: &Collator<usize>,
    ) -> Overlap {
        overlaps(collator, self, other)
    }
}

impl<'a, V, C> Overlaps<(&'a Bound<V>, &'a Bound<V>), C> for (&'a Bound<V>, &'a Bound<V>)
    where C: Collate<Value = V>
{
    fn overlaps(&self, other: &(&'a Bound<V>, &'a Bound<V>), collator: &C) -> Overlap {
        let start = cmp_bound(
            collator,
            self.0.as_ref(),
            other.0.as_ref(),
            Ordering::Greater,
            Ordering::Less,
        );

        let end = cmp_bound(
            collator,
            self.1.as_ref(),
            other.1.as_ref(),
            Ordering::Less,
            Ordering::Greater,
        );

        match (start, end) {
            (Ordering::Equal, Ordering::Equal) => Overlap::Equal,

            (Ordering::Greater, Ordering::Less) => Overlap::Narrow,
            (Ordering::Greater, Ordering::Equal) => Overlap::Narrow,
            (Ordering::Equal, Ordering::Less) => Overlap::Narrow,

            (Ordering::Less, Ordering::Greater) => Overlap::Wide,
            (Ordering::Less, Ordering::Equal) => Overlap::WideLess,
            (Ordering::Equal, Ordering::Greater) => Overlap::WideGreater,

            (Ordering::Less, _) => {
                match cmp_bound(
                    collator,
                    self.1.as_ref(),
                    other.0.as_ref(),
                    Ordering::Less,
                    Ordering::Less,
                ) {
                    Ordering::Less => Overlap::Less,
                    Ordering::Greater | Ordering::Equal => Overlap::WideLess,
                }
            }

            (_, Ordering::Greater) => {
                match cmp_bound(
                    collator,
                    self.0.as_ref(),
                    other.1.as_ref(),
                    Ordering::Greater,
                    Ordering::Greater,
                ) {
                    Ordering::Less | Ordering::Equal => Overlap::WideGreater,
                    Ordering::Greater => Overlap::Greater,
                }
            }
        }
    }
}

macro_rules! overlaps_range {
    ($l:ty, $r:ty, $t:ty) => {
        impl Overlaps<$r, Collator<$t>> for $l {
            fn overlaps(&self, other: &$r, collator: &Collator<$t>) -> Overlap {
                overlaps(collator, self, other)
            }
        }
    };
}

macro_rules! range_overlaps {
    ($t:ty) => {
        overlaps_range!(Range<$t>, Range<$t>, $t);
        overlaps_range!(Range<$t>, RangeFull, $t);
        overlaps_range!(Range<$t>, RangeFrom<$t>, $t);
        overlaps_range!(Range<$t>, RangeInclusive<$t>, $t);
        overlaps_range!(Range<$t>, RangeTo<$t>, $t);
        overlaps_range!(Range<$t>, RangeToInclusive<$t>, $t);

        overlaps_range!(RangeFull, Range<$t>, $t);
        overlaps_range!(RangeFull, RangeFull, $t);
        overlaps_range!(RangeFull, RangeFrom<$t>, $t);
        overlaps_range!(RangeFull, RangeInclusive<$t>, $t);
        overlaps_range!(RangeFull, RangeTo<$t>, $t);
        overlaps_range!(RangeFull, RangeToInclusive<$t>, $t);

        overlaps_range!(RangeFrom<$t>, Range<$t>, $t);
        overlaps_range!(RangeFrom<$t>, RangeFull, $t);
        overlaps_range!(RangeFrom<$t>, RangeFrom<$t>, $t);
        overlaps_range!(RangeFrom<$t>, RangeInclusive<$t>, $t);
        overlaps_range!(RangeFrom<$t>, RangeTo<$t>, $t);
        overlaps_range!(RangeFrom<$t>, RangeToInclusive<$t>, $t);

        overlaps_range!(RangeTo<$t>, Range<$t>, $t);
        overlaps_range!(RangeTo<$t>, RangeFull, $t);
        overlaps_range!(RangeTo<$t>, RangeFrom<$t>, $t);
        overlaps_range!(RangeTo<$t>, RangeInclusive<$t>, $t);
        overlaps_range!(RangeTo<$t>, RangeTo<$t>, $t);
        overlaps_range!(RangeTo<$t>, RangeToInclusive<$t>, $t);
    };
}

range_overlaps!(bool);
range_overlaps!(i8);
range_overlaps!(i16);
range_overlaps!(i32);
range_overlaps!(i64);
range_overlaps!(u8);
range_overlaps!(u16);
range_overlaps!(u32);
range_overlaps!(u64);
range_overlaps!(usize);

#[inline]
fn cmp_bound<'a, C>(
    collator: &'a C,
    left: Bound<&'a C::Value>,
    right: Bound<&'a C::Value>,
    l_ex: Ordering,
    r_ex: Ordering,
) -> Ordering
where
    C: Collate,
{
    match (left, right) {
        (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
        (_, Bound::Unbounded) => l_ex,
        (Bound::Unbounded, _) => r_ex,
        (Bound::Included(this), Bound::Included(that)) => collator.cmp(this, that),
        (Bound::Excluded(this), Bound::Excluded(that)) => collator.cmp(this, that),
        (Bound::Excluded(this), Bound::Included(that)) => match collator.cmp(this, that) {
            Ordering::Equal => l_ex,
            ordering => ordering,
        },
        (Bound::Included(this), Bound::Excluded(that)) => match collator.cmp(this, that) {
            Ordering::Equal => r_ex,
            ordering => ordering,
        },
    }
}

fn overlaps<C, L, R>(collator: &C, left: &L, right: &R) -> Overlap
where
    C: Collate,
    L: RangeBounds<C::Value>,
    R: RangeBounds<C::Value>,
{
    let start = cmp_bound(
        collator,
        left.start_bound(),
        right.start_bound(),
        Ordering::Greater,
        Ordering::Less,
    );

    let end = cmp_bound(
        collator,
        left.end_bound(),
        right.end_bound(),
        Ordering::Less,
        Ordering::Greater,
    );

    match (start, end) {
        (Ordering::Equal, Ordering::Equal) => Overlap::Equal,

        (Ordering::Greater, Ordering::Less) => Overlap::Narrow,
        (Ordering::Greater, Ordering::Equal) => Overlap::Narrow,
        (Ordering::Equal, Ordering::Less) => Overlap::Narrow,

        (Ordering::Less, Ordering::Greater) => Overlap::Wide,
        (Ordering::Less, Ordering::Equal) => Overlap::WideLess,
        (Ordering::Equal, Ordering::Greater) => Overlap::WideGreater,

        (Ordering::Less, _) => {
            match cmp_bound(
                collator,
                left.end_bound(),
                right.start_bound(),
                Ordering::Less,
                Ordering::Less,
            ) {
                Ordering::Less => Overlap::Less,
                Ordering::Greater | Ordering::Equal => Overlap::WideLess,
            }
        }

        (_, Ordering::Greater) => {
            match cmp_bound(
                collator,
                left.start_bound(),
                right.end_bound(),
                Ordering::Greater,
                Ordering::Greater,
            ) {
                Ordering::Less | Ordering::Equal => Overlap::WideGreater,
                Ordering::Greater => Overlap::Greater,
            }
        }
    }
}
