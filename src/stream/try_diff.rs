use std::cmp::Ordering;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::{Fuse, Stream, StreamExt, TryStream};
use pin_project::pin_project;

use crate::Collate;

use super::{try_poll_inner, swap_value};

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

impl<C, E, L, R> Stream for Diff<C, C::Value, L, R>
where
    C: Collate,
    E: std::error::Error,
    Fuse<L>: TryStream<Ok = C::Value, Error = E> + Unpin,
    Fuse<R>: TryStream<Ok = C::Value, Error = E> + Unpin,
{
    type Item = Result<C::Value, E>;

    fn poll_next(self: Pin<&mut Self>, cxt: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            let left_done = if this.left.is_done() {
                true
            } else if this.pending_left.is_none() {
                match try_poll_inner(Pin::new(&mut this.left), this.pending_left, cxt) {
                    Ok(done) => done,
                    Err(cause) => break Some(Err(cause)),
                }
            } else {
                false
            };

            let right_done = if this.right.is_done() {
                true
            } else if this.pending_right.is_none() {
                match try_poll_inner(Pin::new(&mut this.right), this.pending_right, cxt) {
                    Ok(done) => done,
                    Err(cause) => break Some(Err(cause)),
                }
            } else {
                false
            };

            if this.pending_left.is_some() && this.pending_right.is_some() {
                let l_value = this.pending_left.as_ref().unwrap();
                let r_value = this.pending_right.as_ref().unwrap();

                match this.collator.cmp(l_value, r_value) {
                    Ordering::Equal => {
                        // this value is present in the right stream, so drop it
                        swap_value(this.pending_left);
                        swap_value(this.pending_right);
                    }
                    Ordering::Less => {
                        // this value is not present in the right stream, so return it
                        let l_value = swap_value(this.pending_left);
                        break Some(Ok(l_value));
                    }
                    Ordering::Greater => {
                        // this value could be present in the right stream--wait and see
                        swap_value(this.pending_right);
                    }
                }
            } else if right_done && this.pending_left.is_some() {
                let l_value = swap_value(this.pending_left);
                break Some(Ok(l_value));
            } else if left_done {
                break None;
            }
        })
    }
}

/// Compute the difference of two collated [`TryStream`]s,
/// i.e. return the items in `left` that are not in `right`.
/// Both input streams **must** be collated.
/// If either input stream is not collated, the behavior of the output stream is undefined.
pub fn try_diff<C, E, L, R>(collator: C, left: L, right: R) -> Diff<C, C::Value, L, R>
where
    C: Collate,
    E: std::error::Error,
    L: TryStream<Ok = C::Value, Error = E>,
    R: TryStream<Ok = C::Value, Error = E>,
{
    Diff {
        collator,
        left: left.fuse(),
        right: right.fuse(),
        pending_left: None,
        pending_right: None,
    }
}
