use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::actor::Actor;
use crate::clock::{self, Delay, Duration};
use crate::fut::ActorStream;

/// Future for the `timeout` combinator, interrupts computations if it takes
/// more than `timeout`.
///
/// This is created by the `ActorFuture::timeout()` method.
#[pin_project::pin_project]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct StreamTimeout<S>
where
    S: ActorStream,
{
    #[pin]
    stream: S,
    dur: Duration,
    timeout: Option<Delay>,
}

pub fn new<S: ActorStream>(stream: S, timeout: Duration) -> StreamTimeout<S> {
    StreamTimeout {
        stream,
        dur: timeout,
        timeout: None,
    }
}

impl<S: ActorStream> ActorStream for StreamTimeout<S> {
    type Item = Result<S::Item, ()>;
    type Actor = S::Actor;

    fn poll_next(
        self: Pin<&mut Self>,
        act: &mut S::Actor,
        ctx: &mut <S::Actor as Actor>::Context,
        task: &mut Context<'_>,
    ) -> Poll<Option<Result<S::Item, ()>>> {
        let this = self.project();

        match this.stream.poll_next(act, ctx, task) {
            Poll::Ready(Some(res)) => {
                this.timeout.take();
                return Poll::Ready(Some(Ok(res)));
            }
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => (),
        }

        if this.timeout.is_none() {
            *this.timeout = Some(clock::delay_for(*this.dur));
        }

        // check timeout
        match Pin::new(this.timeout.as_mut().unwrap()).poll(task) {
            Poll::Ready(_) => (),
            Poll::Pending => return Poll::Pending,
        }
        this.timeout.take();

        Poll::Ready(Some(Err(())))
    }
}
