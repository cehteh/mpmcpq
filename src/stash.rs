use crate::*;

/// For contention free queue insertion every thread maintains a private
/// 'stash'. When messages are send to the PriorityQueue while it is locked they are stored in
/// this stash. Once the lock is obtained the stashed messages will be moved to the
/// PriorityQueue. This can also be enforced with the 'PriorityQueue::sync()' function.
pub struct Stash<'a, K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    pub(crate) msgs: Vec<Message<K, P>>,
    pq:              &'a PriorityQueue<K, P>,
}

impl<'a, K, P> Stash<'a, K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    /// Creates a new stash. A stash is tied to a priority queue, when the stash becomes
    /// dropped all its remaining temporary messages will be send to the queue.
    pub fn new(pq: &'a PriorityQueue<K, P>) -> Self {
        Stash {
            msgs: Vec::new(),
            pq,
        }
    }
}

impl<K, P> Drop for Stash<'_, K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    fn drop(&mut self) {
        self.pq.sync(self);
    }
}
