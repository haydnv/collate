//! Defines a [`Collate`] trait to standardize collation methods across data types. The provided
//! [`Collator`] struct can be used to collate a collection of items of type `T` where `T: Ord`.
//!
//! [`Collate`] is useful for implementing a B-Tree, or to handle cases where a collator type is
//! more efficient than calling `Ord::cmp` repeatedly, for example when collating localized strings
//! using `rust_icu_ucol`. It's also useful to handle types like complex numbers which do not
//! necessarily have a natural ordering.
//!
//! Use the "stream" feature flag to enable `diff` and `try_diff` functions to compute the
//! difference between two collated `Stream`s, and the `merge` and `try_merge` functions
//! to merge two collated `Stream`s.

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

#[cfg(feature = "stream")]
pub use stream::*;

#[cfg(feature = "stream")]
mod stream;

/// A collator for type `Value`.
pub trait Collate: Sized + Eq {
    type Value;

    /// Return the collation of the `left` value relative to the `right` value.
    fn cmp(&self, left: &Self::Value, right: &Self::Value) -> Ordering;
}

/// A generic collator for any type `T: Ord`.
pub struct Collator<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for Collator<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for Collator<T> {
    fn clone(&self) -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> Copy for Collator<T> {}

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
    /// Return the narrowest [`Overlap`] which contains both `self` and `other`.
    /// Examples:
    /// ```
    /// use collate::Overlap;
    /// assert_eq!(Overlap::Narrow.then(Overlap::Less), Overlap::WideLess);
    /// assert_eq!(Overlap::WideLess.then(Overlap::WideGreater), Overlap::Wide);
    /// assert_eq!(Overlap::Less.then(Overlap::Greater), Overlap::Wide);
    /// assert_eq!(Overlap::Less.then(Overlap::Less), Overlap::Less);
    /// ```
    pub fn then(self, other: Self) -> Self {
        match self {
            Self::Wide => Self::Wide,
            Self::Narrow => match other {
                Self::Less | Self::WideLess => Self::WideLess,
                Self::Narrow | Self::Equal => self,
                Self::Wide => Self::Wide,
                Self::Greater | Self::WideGreater => Self::WideGreater,
            },
            Self::Equal => match other {
                Self::Less | Self::WideLess => Self::WideLess,
                Self::Equal | Self::Narrow | Self::Wide => other,
                Self::Greater | Self::WideGreater => Self::WideGreater,
            },
            Self::Less | Self::WideLess => match other {
                Self::Less => self,
                Self::WideLess | Self::Narrow | Self::Equal => Self::WideLess,
                Self::Wide | Self::WideGreater | Self::Greater => Self::Wide,
            },
            Self::Greater | Self::WideGreater => match other {
                Self::Greater => self,
                Self::WideGreater | Self::Narrow | Self::Equal => Self::WideGreater,
                Self::Wide | Self::WideLess | Self::Less => Self::Wide,
            },
        }
    }
}

/// Range-range comparison methods
pub trait OverlapsRange<T, C: Collate> {
    /// Check whether `other` lies entirely within `self` according to the given `collator`.
    #[inline]
    fn contains(&self, other: &T, collator: &C) -> bool {
        match self.overlaps(other, collator) {
            Overlap::Wide | Overlap::Equal => true,
            _ => false,
        }
    }

    /// Check whether `other` lies partially within `self` according to the given `collator`.
    #[inline]
    fn contains_partial(&self, other: &T, collator: &C) -> bool {
        match self.overlaps(other, collator) {
            Overlap::Narrow | Overlap::Equal => true,
            Overlap::WideLess | Overlap::Wide | Overlap::WideGreater => true,
            _ => false,
        }
    }

    /// Check whether `self` overlaps `other` according to the given `collator`.
    ///
    /// Examples:
    /// ```
    /// use collate::{Collate, Collator, Overlap, OverlapsRange};
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

type BorrowBounds<'a, V> = (&'a Bound<V>, &'a Bound<V>);

impl<'a, C> OverlapsRange<BorrowBounds<'a, C::Value>, C> for BorrowBounds<'a, C::Value>
where
    C: Collate,
{
    fn overlaps(&self, other: &BorrowBounds<'a, C::Value>, collator: &C) -> Overlap {
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
    ($l:ty, $r:ty) => {
        impl<C: Collate> OverlapsRange<$r, C> for $l {
            fn overlaps(&self, other: &$r, collator: &C) -> Overlap {
                overlaps(collator, self, other)
            }
        }
    };
}

overlaps_range!(Range<C::Value>, (Bound<C::Value>, Bound<C::Value>));
overlaps_range!(Range<C::Value>, Range<C::Value>);
overlaps_range!(Range<C::Value>, RangeFull);
overlaps_range!(Range<C::Value>, RangeFrom<C::Value>);
overlaps_range!(Range<C::Value>, RangeInclusive<C::Value>);
overlaps_range!(Range<C::Value>, RangeTo<C::Value>);
overlaps_range!(Range<C::Value>, RangeToInclusive<C::Value>);

overlaps_range!(RangeFull, (Bound<C::Value>, Bound<C::Value>));
overlaps_range!(RangeFull, Range<C::Value>);
overlaps_range!(RangeFull, RangeFull);
overlaps_range!(RangeFull, RangeFrom<C::Value>);
overlaps_range!(RangeFull, RangeInclusive<C::Value>);
overlaps_range!(RangeFull, RangeTo<C::Value>);
overlaps_range!(RangeFull, RangeToInclusive<C::Value>);

overlaps_range!(RangeFrom<C::Value>, (Bound<C::Value>, Bound<C::Value>));
overlaps_range!(RangeFrom<C::Value>, Range<C::Value>);
overlaps_range!(RangeFrom<C::Value>, RangeFull);
overlaps_range!(RangeFrom<C::Value>, RangeFrom<C::Value>);
overlaps_range!(RangeFrom<C::Value>, RangeInclusive<C::Value>);
overlaps_range!(RangeFrom<C::Value>, RangeTo<C::Value>);
overlaps_range!(RangeFrom<C::Value>, RangeToInclusive<C::Value>);

overlaps_range!(RangeTo<C::Value>, (Bound<C::Value>, Bound<C::Value>));
overlaps_range!(RangeTo<C::Value>, Range<C::Value>);
overlaps_range!(RangeTo<C::Value>, RangeFull);
overlaps_range!(RangeTo<C::Value>, RangeFrom<C::Value>);
overlaps_range!(RangeTo<C::Value>, RangeInclusive<C::Value>);
overlaps_range!(RangeTo<C::Value>, RangeTo<C::Value>);
overlaps_range!(RangeTo<C::Value>, RangeToInclusive<C::Value>);

overlaps_range!(
    (Bound<C::Value>, Bound<C::Value>),
    (Bound<C::Value>, Bound<C::Value>)
);
overlaps_range!((Bound<C::Value>, Bound<C::Value>), Range<C::Value>);
overlaps_range!((Bound<C::Value>, Bound<C::Value>), RangeFull);
overlaps_range!((Bound<C::Value>, Bound<C::Value>), RangeFrom<C::Value>);
overlaps_range!((Bound<C::Value>, Bound<C::Value>), RangeInclusive<C::Value>);
overlaps_range!((Bound<C::Value>, Bound<C::Value>), RangeTo<C::Value>);
overlaps_range!(
    (Bound<C::Value>, Bound<C::Value>),
    RangeToInclusive<C::Value>
);

/// Range-value comparison methods
pub trait OverlapsValue<V, C: Collate> {
    /// Return `true` if this range contains `value` according to `collator`.
    fn contains_value(&self, value: &V, collator: &C) -> bool {
        match self.overlaps_value(value, collator) {
            Overlap::Less | Overlap::Greater => false,
            _ => true,
        }
    }

    /// Return `true` if this range overlaps `value` according to `collator`.
    fn overlaps_value(&self, value: &V, collator: &C) -> Overlap;
}

macro_rules! overlaps_value {
    ($t:ty) => {
        impl<C> OverlapsValue<C::Value, C> for $t
        where
            C: Collate,
        {
            fn overlaps_value(&self, value: &C::Value, collator: &C) -> Overlap {
                overlaps_value(self, value, collator)
            }
        }
    };
}

overlaps_value!((Bound<C::Value>, Bound<C::Value>));
overlaps_value!(Range<C::Value>);
overlaps_value!(RangeFull);
overlaps_value!(RangeFrom<C::Value>);
overlaps_value!(RangeInclusive<C::Value>);
overlaps_value!(RangeTo<C::Value>);
overlaps_value!(RangeToInclusive<C::Value>);

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

#[inline]
fn overlaps_value<C: Collate, R: RangeBounds<C::Value>>(
    range: &R,
    value: &C::Value,
    collator: &C,
) -> Overlap {
    let start = match range.start_bound() {
        Bound::Unbounded => Ordering::Less,
        Bound::Included(start) => collator.cmp(start, value),
        Bound::Excluded(start) => match collator.cmp(start, value) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater | Ordering::Equal => Ordering::Greater,
        },
    };

    let end = match range.end_bound() {
        Bound::Unbounded => Ordering::Greater,
        Bound::Included(end) => collator.cmp(end, value),
        Bound::Excluded(end) => match collator.cmp(end, value) {
            Ordering::Greater => Ordering::Greater,
            Ordering::Less | Ordering::Equal => Ordering::Less,
        },
    };

    match (start, end) {
        (start, Ordering::Less) => {
            debug_assert_eq!(start, Ordering::Less);
            Overlap::Less
        }
        (Ordering::Greater, end) => {
            debug_assert_eq!(end, Ordering::Greater);
            Overlap::Greater
        }

        (Ordering::Equal, Ordering::Equal) => Overlap::Equal,

        (Ordering::Equal, Ordering::Greater) => Overlap::WideGreater,
        (Ordering::Less, Ordering::Greater) => Overlap::Wide,
        (Ordering::Less, Ordering::Equal) => Overlap::WideLess,
    }
}
