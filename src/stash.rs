use crate::*;

/// For contention free queue insertion every thread maintains a private
/// 'stash'. When messages are send to the PriorityQueue while it is locked they are stored in
/// this stash. Once the lock is obtained the stashed messages will be moved to the
/// PriorityQueue. This can also be enforced with the 'PriorityQueue::sync()' function.
pub struct Stash<'a, M, P>
where
    M: Send,
    P: PartialOrd + Ord,
{
    pub(crate) msgs: Vec<Message<M, P>>,
    pq:              Option<&'a PriorityQueue<M, P>>,
}

impl<'a, M, P> Stash<'a, M, P>
where
    M: Send,
    P: PartialOrd + Ord,
{
    /// Creates a new stash. A stash is tied to a priority queue, when the stash becomes
    /// dropped all its remaining temporary messages will be sent to the queue.
    pub fn new(pq: &'a PriorityQueue<M, P>) -> Self {
        Stash {
            msgs: Vec::new(),
            pq:   Some(pq),
        }
    }

    /// Creates a new stash. This Stash has no reference to a priority queue, messages left
    /// here at drop time become discarded.
    pub fn without_priority_queue() -> Self {
        Stash {
            msgs: Vec::new(),
            pq:   None,
        }
    }

    /// Returns true when the Stash contains no messages.
    pub fn is_empty(&self) -> bool {
        self.msgs.is_empty()
    }

    /// Returns the number of messages in the stash.
    pub fn len(&self) -> usize {
        self.msgs.len()
    }

    /// Returns the number of messages the stash can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.msgs.capacity()
    }
}

impl<M, P> Drop for Stash<'_, M, P>
where
    M: Send,
    P: PartialOrd + Ord,
{
    fn drop(&mut self) {
        if let Some(pq) = self.pq {
            pq.sync(self);
        }
    }
}
