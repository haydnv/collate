use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::Deref;

pub trait Collate {
    type Value;

    fn bisect_left<V: Deref<Target = [Self::Value]>>(&self, slice: &[V], key: &[Self::Value]) -> usize {
        if slice.is_empty() || key.is_empty() {
            0
        } else {
            bisect_left(slice, |at| self.compare_slice(at, key))
        }
    }

    fn bisect_right<V: Deref<Target = [Self::Value]>>(&self, slice: &[V], key: &[Self::Value]) -> usize {
        if slice.is_empty() {
            0
        } else if key.is_empty() {
            slice.len()
        } else {
            bisect_right(slice, |at| self.compare_slice(at, key))
        }
    }

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering;

    fn compare_slice<L: Deref<Target = [Self::Value]>, R: Deref<Target = [Self::Value]>>(
        &self,
        left: L,
        right: R,
    ) -> Ordering {
        use Ordering::*;

        for i in 0..Ord::min(left.len(), right.len()) {
            match self.compare(&left[i], &right[i]) {
                Equal => {}
                rel => return rel,
            };
        }

        if left.is_empty() && !right.is_empty() {
            Less
        } else if !left.is_empty() && right.is_empty() {
            Greater
        } else {
            Equal
        }
    }

    fn is_sorted<V: Deref<Target = [Self::Value]>>(&self, slice: &[V]) -> bool {
        if slice.len() < 2 {
            return true;
        }

        let order = self.compare_slice(slice[1].deref(),slice[0].deref());
        for i in 1..slice.len() {
            let rel = self.compare_slice(slice[i].deref(), slice[i - 1].deref());
            if rel != order && rel != Ordering::Equal {
                return false;
            }
        }

        true
    }
}

#[derive(Default)]
pub struct Collator<T> {
    phantom: PhantomData<T>,
}

impl<T: Ord> Collate for Collator<T> {
    type Value = T;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        left.cmp(right)
    }
}

fn bisect_left<'a, V: 'a, W: Deref<Target = [V]>, F: Fn(&'a [V]) -> Ordering>(
    slice: &'a [W],
    cmp: F,
) -> usize {
    let mut start = 0;
    let mut end = slice.len();

    while start < end {
        let mid = (start + end) / 2;

        if cmp(&slice[mid]) == Ordering::Less {
            start = mid + 1;
        } else {
            end = mid;
        }
    }

    start
}

fn bisect_right<'a, V: 'a, W: Deref<Target = [V]>, F: Fn(&'a [V]) -> Ordering>(
    slice: &'a [W],
    cmp: F,
) -> usize {
    let mut start = 0;
    let mut end = slice.len();

    while start < end {
        let mid = (start + end) / 2;

        if cmp(&slice[mid]) == Ordering::Greater {
            end = mid;
        } else {
            start = mid + 1;
        }
    }

    end
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;
    use super::*;

    struct Key {
        inner: Vec<i32>
    }

    impl Deref for Key {
        type Target = [i32];

        fn deref(&self) -> &[i32] {
            &self.inner
        }
    }

    struct Block {
        keys: Vec<Key>
    }

    impl Deref for Block {
        type Target = [Key];

        fn deref(&self) -> &[Key] {
            &self.keys
        }
    }

    #[test]
    fn test_bisect() {
        let block = Block {
            keys: vec![
                Key { inner: vec![0, -1, 1] },
                Key { inner: vec![1, 0, 2, 2] },
                Key { inner: vec![1, 1, 0] },
                Key { inner: vec![1, 1, 0] },
                Key { inner: vec![2, 0, -1] },
                Key { inner: vec![2, 1] },
            ]
        };

        let collator = Collator::<i32>::default();
        assert!(collator.is_sorted(&block));

        assert_eq!(collator.bisect_left(&block, &[0, 0, 0]), 1);
        assert_eq!(collator.bisect_right(&block, &[0, 0, 0]), 1);

        assert_eq!(collator.bisect_left(&block, &[0]), 0);
        assert_eq!(collator.bisect_right(&block, &[0]), 1);

        assert_eq!(collator.bisect_left(&block, &[1, 1, 0]), 2);
        assert_eq!(collator.bisect_right(&block, &[1, 1, 0]), 4);

        assert_eq!(collator.bisect_left(&block, &[1, 1, 0, -1]), 2);
        assert_eq!(collator.bisect_right(&block, &[1, 1, 0, -1]), 4);
        assert_eq!(collator.bisect_right(&block, &[1, 1, 0, 1]), 4);

        assert_eq!(collator.bisect_left(&block, &[3]), 6);
        assert_eq!(collator.bisect_right(&block, &[3]), 6);
    }
}
