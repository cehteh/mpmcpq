use std::cell::RefCell;

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
    pub(crate) msgs: RefCell<Vec<Message<K, P>>>,
    pq:              Option<&'a PriorityQueue<K, P>>,
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
            msgs: RefCell::new(Vec::new()),
            pq:   Some(pq),
        }
    }

    /// Creates a new stash. This Stash has no reference to a priority queue, messages left
    /// here at drop time become discarded.
    pub fn new_without_priority_queue() -> Self {
        Stash {
            msgs: RefCell::new(Vec::new()),
            pq:   None,
        }
    }
}

impl<K, P> Drop for Stash<'_, K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    fn drop(&mut self) {
        if let Some(pq) = self.pq {
            pq.sync(self);
        }
    }
}
