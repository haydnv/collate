use std::cmp::Ordering;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::{Fuse, Stream, StreamExt, TryStream};
use pin_project::pin_project;

use crate::Collate;

use super::{try_poll_inner, swap_value};

/// The stream returned by [`merge`].
/// The implementation of this stream is based on
/// [`stream::select`](https://github.com/rust-lang/futures-rs/blob/master/futures-util/src/stream/select.rs).
#[pin_project]
pub struct TryMerge<C, T, L, R> {
    collator: C,

    #[pin]
    left: Fuse<L>,
    #[pin]
    right: Fuse<R>,

    pending_left: Option<T>,
    pending_right: Option<T>,
}

impl<C, E, L, R> Stream for TryMerge<C, C::Value, L, R>
where
    C: Collate,
    Fuse<L>: TryStream<Ok = C::Value, Error = E> + Unpin,
    Fuse<R>: TryStream<Ok = C::Value, Error = E> + Unpin,
{
    type Item = Result<C::Value, E>;

    fn poll_next(self: Pin<&mut Self>, cxt: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let left_done = if this.left.is_done() {
            true
        } else if this.pending_left.is_none() {
            match try_poll_inner(this.left, this.pending_left, cxt) {
                Err(cause) => return Poll::Ready(Some(Err(cause))),
                Ok(done) => done,
            }
        } else {
            false
        };

        let right_done = if this.right.is_done() {
            true
        } else if this.pending_right.is_none() {
            match try_poll_inner(this.right, this.pending_right, cxt) {
                Err(cause) => return Poll::Ready(Some(Err(cause))),
                Ok(done) => done,
            }
        } else {
            false
        };

        if this.pending_left.is_some() && this.pending_right.is_some() {
            let l_value = this.pending_left.as_ref().unwrap();
            let r_value = this.pending_right.as_ref().unwrap();

            match this.collator.cmp(l_value, r_value) {
                Ordering::Equal => {
                    let l_value = swap_value(this.pending_left);
                    let _r_value = swap_value(this.pending_right);
                    Poll::Ready(Some(Ok(l_value)))
                }
                Ordering::Less => {
                    let l_value = swap_value(this.pending_left);
                    Poll::Ready(Some(Ok(l_value)))
                }
                Ordering::Greater => {
                    let r_value = swap_value(this.pending_right);
                    Poll::Ready(Some(Ok(r_value)))
                }
            }
        } else if right_done && this.pending_left.is_some() {
            let l_value = swap_value(this.pending_left);
            Poll::Ready(Some(Ok(l_value)))
        } else if left_done && this.pending_right.is_some() {
            let r_value = swap_value(this.pending_right);
            Poll::Ready(Some(Ok(r_value)))
        } else if left_done && right_done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

/// Merge two collated [`TryStream`]s into one using the given `collator`.
/// Both input streams **must** be collated and have the same error type.
/// If either input stream is not collated, the order of the output stream is undefined.
pub fn try_merge<C, E, L, R>(collator: C, left: L, right: R) -> TryMerge<C, C::Value, L, R>
where
    C: Collate,
    E: std::error::Error,
    L: TryStream<Ok = C::Value, Error = E>,
    R: TryStream<Ok = C::Value, Error = E>,
{
    TryMerge {
        collator,
        left: left.fuse(),
        right: right.fuse(),
        pending_left: None,
        pending_right: None,
    }
}
