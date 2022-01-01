use std::sync::atomic;

use crate::*;

/// Wraps a Message, when dropped and the queue is empty it sends a final 'Drained' message
/// to notify that there is no further work in progress.
#[derive(Debug)]
pub struct ReceiveGuard<'a, K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    msg: Message<K, P>,
    pq:  &'a PriorityQueue<K, P>,
}

impl<'a, K, P> ReceiveGuard<'a, K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    pub(crate) fn new(msg: Message<K, P>, pq: &'a PriorityQueue<K, P>) -> Self {
        ReceiveGuard { msg, pq }
    }

    /// Returns a reference to the contained message.
    pub fn message(&self) -> &Message<K, P> {
        &self.msg
    }

    /// Takes the 'Message' entry out of a ReceiveGuard, drop the guard (and by that, may send
    /// the 'Drained' message).
    pub fn into_message(mut self) -> Message<K, P> {
        std::mem::take(&mut self.msg)
    }
}

impl<K, P> Drop for ReceiveGuard<'_, K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    fn drop(&mut self) {
        if self.pq.in_progress.fetch_sub(1, atomic::Ordering::SeqCst) == 1 {
            self.pq.send_drained()
        }
    }
}
