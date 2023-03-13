use std::cmp::Ordering;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::{Fuse, Stream, StreamExt};
use pin_project::pin_project;

use crate::Collate;

/// The stream type returned by [`diff`].
/// The implementation of this stream is based on
/// [`stream::select`](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/select.rs).
#[pin_project]
pub struct Diff<C, T, L, R> {
    collator: C,

    #[pin]
    left: Fuse<L>,
    #[pin]
    right: Fuse<R>,

    pending_left: Option<T>,
    pending_right: Option<T>,
}

impl<C, L, R> Diff<C, C::Value, L, R>
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

impl<C, L, R> Stream for Diff<C, C::Value, L, R>
where
    C: Collate,
    L: Stream<Item = C::Value> + Unpin,
    R: Stream<Item = C::Value> + Unpin,
{
    type Item = C::Value;

    fn poll_next(self: Pin<&mut Self>, cxt: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            let left_done = if this.left.is_done() {
                true
            } else if this.pending_left.is_none() {
                Self::poll_inner(Pin::new(&mut this.left), this.pending_left, cxt)
            } else {
                false
            };

            let right_done = if this.right.is_done() {
                true
            } else if this.pending_right.is_none() {
                Self::poll_inner(Pin::new(&mut this.right), this.pending_right, cxt)
            } else {
                false
            };

            if this.pending_left.is_some() && this.pending_right.is_some() {
                let l_value = this.pending_left.as_ref().unwrap();
                let r_value = this.pending_right.as_ref().unwrap();

                match this.collator.cmp(l_value, r_value) {
                    Ordering::Equal => {
                        // this value is present in the right stream, so drop it
                        Self::swap_value(this.pending_left);
                        Self::swap_value(this.pending_right);
                    }
                    Ordering::Less => {
                        // this value is not present in the right stream, so return it
                        let l_value = Self::swap_value(this.pending_left);
                        break Some(l_value);
                    }
                    Ordering::Greater => {
                        // this value could be present in the right stream--wait and see
                        Self::swap_value(this.pending_right);
                    }
                }
            } else if right_done && this.pending_left.is_some() {
                let l_value = Self::swap_value(this.pending_left);
                break Some(l_value);
            } else if left_done {
                break None;
            }
        })
    }
}

/// Compute the difference of two collated [`Streams`,
/// i.e. return the items in `left` that are not in `right`.
/// Both input streams **must** be collated.
/// If either input stream is not collated, the behavior of the output stream is undefined.
pub fn diff<C, L, R>(collator: C, left: L, right: R) -> Diff<C, C::Value, L, R>
where
    C: Collate,
    L: Stream<Item = C::Value>,
    R: Stream<Item = C::Value>,
{
    Diff {
        collator,
        left: left.fuse(),
        right: right.fuse(),
        pending_left: None,
        pending_right: None,
    }
}
