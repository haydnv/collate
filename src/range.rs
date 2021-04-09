use std::borrow::Borrow;
use std::ops::{self, Bound};

/// A range for use with the `Collate` trait.
#[derive(Clone)]
pub struct Range<V, B: Borrow<[V]>> {
    prefix: B,
    start: Bound<V>,
    end: Bound<V>,
}

impl<V, B: Borrow<[V]>> Range<V, B> {
    pub fn len(&self) -> usize {
        self.prefix().len() + 1
    }

    pub fn prefix(&'_ self) -> &'_ [V] {
        self.prefix.borrow()
    }

    pub fn start(&'_ self) -> &'_ Bound<V> {
        &self.start
    }

    pub fn end(&'_ self) -> &'_ Bound<V> {
        &self.end
    }
}

impl<V, B: Borrow<[V]>> From<(B, ops::Range<V>)> for Range<V, B> {
    fn from(tuple: (B, ops::Range<V>)) -> Self {
        let (prefix, suffix) = tuple;
        let ops::Range { start, end } = suffix;
        Self {
            prefix,
            start: Bound::Included(start),
            end: Bound::Excluded(end),
        }
    }
}

impl<V, B: Borrow<[V]>> From<(B, ops::RangeFrom<V>)> for Range<V, B> {
    fn from(tuple: (B, ops::RangeFrom<V>)) -> Self {
        let (prefix, suffix) = tuple;
        let ops::RangeFrom { start } = suffix;
        Self {
            prefix,
            start: Bound::Included(start),
            end: Bound::Unbounded,
        }
    }
}

impl<V, B: Borrow<[V]>> From<(B, ops::RangeTo<V>)> for Range<V, B> {
    fn from(tuple: (B, ops::RangeTo<V>)) -> Self {
        let (prefix, suffix) = tuple;
        let ops::RangeTo { end } = suffix;
        Self {
            prefix,
            start: Bound::Unbounded,
            end: Bound::Excluded(end),
        }
    }
}

impl<V, B: Borrow<[V]>> From<(B, ops::Bound<V>, ops::Bound<V>)> for Range<V, B> {
    fn from(tuple: (B, ops::Bound<V>, ops::Bound<V>)) -> Self {
        let (prefix, start, end) = tuple;
        Self { prefix, start, end }
    }
}
