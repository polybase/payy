use crate::{App, Solid, event::SolidEvent};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::Stream;

impl<A: App> Stream for Solid<A> {
    type Item = SolidEvent<A::P, A::State>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut events = self.shared.events.lock();
        if let Some(event) = events.pop_front() {
            return Poll::Ready(Some(event));
        }
        let mut state = self.shared.state.lock();
        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}
