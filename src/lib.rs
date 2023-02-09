//! Defines a [`Collate`] trait to standardize collation methods across data types. The provided
//! [`Collator`] struct can be used to collate a collection of slices of type `T` where `T: Ord`.
//!
//! [`Collate`] is useful for implementing a B-Tree, or to handle cases where a collator type is
//! more efficient than calling `Ord::cmp` repeatedly, for example when collating localized strings
//! using `rust_icu_ucol`. It's also useful to handle types like complex numbers which do not
//! necessarily have a natural ordering.

use std::cmp::Ordering;
use std::marker::PhantomData;

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
