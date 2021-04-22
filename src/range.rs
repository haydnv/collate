use std::borrow::Borrow;
use std::ops::{self, Bound};

use super::Collate;

/// A range for use with the `Collate` trait.
#[derive(Clone, Eq, PartialEq)]
pub struct Range<V, B> {
    prefix: B,
    start: Bound<V>,
    end: Bound<V>,
}

impl<V: Eq, B: Borrow<[V]>> Range<V, B> {
    pub fn contains<C: Collate<Value = V>>(&self, other: &Self, collator: &C) -> bool {
        if other.prefix.borrow().len() < self.prefix.borrow().len() {
            return false;
        }

        if &other.prefix.borrow()[..self.prefix.borrow().len()] != &self.prefix.borrow()[..] {
            return false;
        }

        use std::cmp::Ordering::*;
        if other.prefix.borrow().len() == self.prefix.borrow().len() {
            match &self.start {
                Bound::Unbounded => {}
                Bound::Included(outer) => match &other.start {
                    Bound::Unbounded => return false,
                    Bound::Included(inner) => {
                        if collator.compare(inner, outer) == Less {
                            return false;
                        }
                    }
                    Bound::Excluded(inner) => {
                        if collator.compare(inner, outer) != Greater {
                            return false;
                        }
                    }
                },
                Bound::Excluded(outer) => match &other.start {
                    Bound::Unbounded => return false,
                    Bound::Included(inner) => {
                        if collator.compare(inner, outer) != Greater {
                            return false;
                        }
                    }
                    Bound::Excluded(inner) => {
                        if collator.compare(inner, outer) == Less {
                            return false;
                        }
                    }
                },
            }
        } else {
            let value = &other.prefix.borrow()[self.prefix.borrow().len()];

            match &self.start {
                Bound::Unbounded => {}
                Bound::Included(outer) => {
                    if collator.compare(value, outer) == Less {
                        return false;
                    }
                }
                Bound::Excluded(outer) => {
                    if collator.compare(value, outer) != Greater {
                        return false;
                    }
                }
            }

            match &self.end {
                Bound::Unbounded => {}
                Bound::Included(outer) => {
                    if collator.compare(value, outer) == Greater {
                        return false;
                    }
                }
                Bound::Excluded(outer) => {
                    if collator.compare(value, outer) != Less {
                        return false;
                    }
                }
            }
        }

        true
    }
}

impl<V> Default for Range<V, Vec<V>> {
    fn default() -> Self {
        Self {
            prefix: vec![],
            start: Bound::Unbounded,
            end: Bound::Unbounded,
        }
    }
}

impl<V, B: Borrow<[V]>> Range<V, B> {
    pub fn new(prefix: B, range: ops::Range<V>) -> Self {
        let ops::Range { start, end } = range;

        Self {
            prefix,
            start: Bound::Included(start),
            end: Bound::Excluded(end),
        }
    }

    pub fn with_prefix(prefix: B) -> Self {
        Self {
            prefix,
            start: Bound::Unbounded,
            end: Bound::Unbounded,
        }
    }

    pub fn into_inner(self) -> (B, Bound<V>, Bound<V>) {
        (self.prefix, self.start, self.end)
    }

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

impl<V, B> From<(B, ops::Range<V>)> for Range<V, B> {
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

impl<V, B> From<(B, ops::RangeFrom<V>)> for Range<V, B> {
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

impl<V, B> From<(B, ops::RangeTo<V>)> for Range<V, B> {
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

impl<V, B> From<(B, ops::Bound<V>, ops::Bound<V>)> for Range<V, B> {
    fn from(tuple: (B, ops::Bound<V>, ops::Bound<V>)) -> Self {
        let (prefix, start, end) = tuple;
        Self { prefix, start, end }
    }
}
