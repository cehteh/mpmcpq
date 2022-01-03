use std::sync::atomic;

use crate::*;

/// Wraps a Message, when dropped and the queue is empty it sends a final 'Drained' message
/// to notify that there is no further work in progress.
#[derive(Debug)]
pub struct ReceiveGuard<'a, M, P>
where
    M: Send,
    P: PartialOrd + Ord,
{
    msg: Message<M, P>,
    pq:  &'a PriorityQueue<M, P>,
}

impl<'a, M, P> ReceiveGuard<'a, M, P>
where
    M: Send,
    P: PartialOrd + Ord,
{
    pub(crate) fn new(msg: Message<M, P>, pq: &'a PriorityQueue<M, P>) -> Self {
        ReceiveGuard { msg, pq }
    }

    /// Returns a reference to the contained message.
    pub fn message(&self) -> &Message<M, P> {
        &self.msg
    }

    /// Takes the Message out of a ReceiveGuard, drop the guard (and by that, may send
    /// the 'Drained' message).
    pub fn into_message(mut self) -> Message<M, P> {
        std::mem::take(&mut self.msg)
    }
}

impl<M, P> Drop for ReceiveGuard<'_, M, P>
where
    M: Send,
    P: PartialOrd + Ord,
{
    fn drop(&mut self) {
        if self.pq.in_progress.fetch_sub(1, atomic::Ordering::SeqCst) == 1 {
            self.pq.send_drained()
        }
    }
}
