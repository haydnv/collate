use std::cmp::Ordering;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::{Fuse, Stream, StreamExt};
use pin_project::pin_project;

use super::Collate;

/// The stream returned by [`merge`].
/// The implementation of this stream is based on
/// [`stream::select`](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/select.rs).
#[pin_project]
pub struct Merge<C, T, L, R> {
    collator: C,

    #[pin]
    left: Fuse<L>,
    #[pin]
    right: Fuse<R>,

    pending_left: Option<T>,
    pending_right: Option<T>,
}

impl<C, L, R> Merge<C, C::Value, L, R>
where
    C: Collate,
    L: Stream<Item = C::Value>,
    R: Stream<Item = C::Value>,
{
    fn poll_inner<S: Stream<Item = C::Value>>(
        stream: Pin<&mut Fuse<S>>,
        pending: &mut Option<C::Value>,
        cxt: &mut Context,
    ) -> bool {
        match stream.poll_next(cxt) {
            Poll::Pending => false,
            Poll::Ready(Some(value)) => {
                *pending = Some(value);
                false
            }
            Poll::Ready(None) => true,
        }
    }

    fn swap_value(pending: &mut Option<C::Value>) -> C::Value {
        debug_assert!(pending.is_some());

        let mut value: Option<C::Value> = None;
        mem::swap(pending, &mut value);
        value.unwrap()
    }
}

impl<C, L, R> Stream for Merge<C, C::Value, L, R>
where
    C: Collate,
    L: Stream<Item = C::Value> + Unpin,
    R: Stream<Item = C::Value> + Unpin,
{
    type Item = C::Value;

    fn poll_next(self: Pin<&mut Self>, cxt: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let left_done = if this.left.is_done() {
            true
        } else if this.pending_left.is_none() {
            Self::poll_inner(this.left, this.pending_left, cxt)
        } else {
            false
        };

        let right_done = if this.right.is_done() {
            true
        } else if this.pending_right.is_none() {
            Self::poll_inner(this.right, this.pending_right, cxt)
        } else {
            false
        };

        if this.pending_left.is_some() && this.pending_right.is_some() {
            let l_value = this.pending_left.as_ref().unwrap();
            let r_value = this.pending_right.as_ref().unwrap();

            match this.collator.cmp(l_value, r_value) {
                Ordering::Equal => {
                    let l_value = Self::swap_value(this.pending_left);
                    let _r_value = Self::swap_value(this.pending_right);
                    Poll::Ready(Some(l_value))
                }
                Ordering::Less => {
                    let l_value = Self::swap_value(this.pending_left);
                    Poll::Ready(Some(l_value))
                }
                Ordering::Greater => {
                    let r_value = Self::swap_value(this.pending_right);
                    Poll::Ready(Some(r_value))
                }
            }
        } else if right_done && this.pending_left.is_some() {
            let l_value = Self::swap_value(this.pending_left);
            Poll::Ready(Some(l_value))
        } else if left_done && this.pending_right.is_some() {
            let r_value = Self::swap_value(this.pending_right);
            Poll::Ready(Some(r_value))
        } else if left_done && right_done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

/// Merge two collated streams into one using the given `collator`.
/// Both input streams **must** be collated.
/// If either input stream is not collated, the order of the output stream is undefined.
pub fn merge<C, L, R>(collator: C, left: L, right: R) -> Merge<C, C::Value, L, R>
where
    C: Collate,
    L: Stream<Item = C::Value>,
    R: Stream<Item = C::Value>,
{
    Merge {
        collator,
        left: left.fuse(),
        right: right.fuse(),
        pending_left: None,
        pending_right: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Collator;
    use futures::stream;

    #[tokio::test]
    async fn test_merge() {
        let collator = Collator::<u32>::default();

        let left = vec![1, 3, 5, 7, 8, 9, 20];
        let right = vec![2, 4, 6, 8, 9, 10, 11, 12];

        let expected = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 20];
        let actual = merge(collator, stream::iter(left), stream::iter(right))
            .collect::<Vec<u32>>()
            .await;

        assert_eq!(expected, actual);
    }
}
