use std::cmp::Ordering;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use futures::stream::{Fuse, Stream, StreamExt, TryStream};
use pin_project::pin_project;

use crate::Collate;

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
        #[cfg(feature = "logging")]
        log::debug!("TryMerge::poll_next");

        let this = self.project();

        let left_done = if this.left.is_done() {
            true
        } else if this.pending_left.is_none() {
            #[cfg(feature = "logging")]
            log::debug!("TryMerge::poll_next left");

            match ready!(this.left.try_poll_next(cxt)) {
                Some(Ok(value)) => {
                    *this.pending_left = Some(value);
                    false
                }
                Some(Err(cause)) => return Poll::Ready(Some(Err(cause))),
                None => true,
            }
        } else {
            false
        };

        let right_done = if this.right.is_done() {
            true
        } else if this.pending_right.is_none() {
            #[cfg(feature = "logging")]
            log::debug!("TryMerge::poll_next right");

            match ready!(this.right.try_poll_next(cxt)) {
                Some(Ok(value)) => {
                    *this.pending_right = Some(value);
                    false
                }
                Some(Err(cause)) => return Poll::Ready(Some(Err(cause))),
                None => true,
            }
        } else {
            false
        };

        let value = if this.pending_left.is_some() && this.pending_right.is_some() {
            let l_value = this.pending_left.as_ref().unwrap();
            let r_value = this.pending_right.as_ref().unwrap();

            match this.collator.cmp(l_value, r_value) {
                Ordering::Equal => {
                    this.pending_right.take();
                    this.pending_left.take()
                }
                Ordering::Less => this.pending_left.take(),
                Ordering::Greater => this.pending_right.take(),
            }
        } else if right_done && this.pending_left.is_some() {
            this.pending_left.take()
        } else if left_done && this.pending_right.is_some() {
            this.pending_right.take()
        } else if left_done && right_done {
            None
        } else {
            unreachable!("both streams to merge are still pending")
        };

        Poll::Ready(value.map(Ok))
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
