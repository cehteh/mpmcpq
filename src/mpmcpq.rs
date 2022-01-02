use std::collections::BinaryHeap;
use std::sync::atomic::{self, AtomicBool, AtomicUsize};

use parking_lot::{Condvar, Mutex};

use crate::*;

/// A queue which orders entrys by priority
#[derive(Debug)]
pub struct PriorityQueue<K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    heap:                   Mutex<BinaryHeap<Message<K, P>>>,
    pub(crate) in_progress: AtomicUsize,
    is_drained:             AtomicBool,
    notify:                 Condvar,
}

impl<K, P> Default for PriorityQueue<K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

enum Notify {
    None,
    One,
    All,
}

impl<K, P> PriorityQueue<K, P>
where
    K: Send,
    P: PartialOrd + Ord,
{
    /// Create a new PriorityQueue
    pub fn new() -> PriorityQueue<K, P> {
        PriorityQueue {
            heap:        Mutex::new(BinaryHeap::new()),
            in_progress: AtomicUsize::new(0),
            is_drained:  AtomicBool::new(true),
            notify:      Condvar::new(),
        }
    }

    /// Inserts all elements from the stash to the PriorityQueue, empties stash.
    pub fn sync(&self, stash: &Stash<K, P>) {
        let mut notify = Notify::None;

        let mut msgs = stash.msgs.borrow_mut();

        if !msgs.is_empty() {
            if msgs.len() == 1 {
                notify = Notify::One;
            } else {
                notify = Notify::All;
            }

            let mut lock = self.heap.lock();
            msgs.drain(..).for_each(|e| {
                lock.push(e);
            });
        }

        match notify {
            Notify::None => {}
            Notify::One => {
                self.notify.notify_one();
            }
            Notify::All => {
                self.notify.notify_all();
            }
        }
    }

    fn send_with_stash(&self, entry: Message<K, P>, stash: &Stash<K, P>) {
        let mut notify = Notify::None;

        let mut msgs = stash.msgs.borrow_mut();

        if let Some(mut lock) = self.heap.try_lock() {
            if msgs.is_empty() {
                notify = Notify::One;
            } else {
                notify = Notify::All;
                msgs.drain(..).for_each(|e| {
                    lock.push(e);
                });
            }
            lock.push(entry);
        } else {
            msgs.push(entry);
        }

        match notify {
            Notify::None => {}
            Notify::One => {
                self.notify.notify_one();
            }
            Notify::All => {
                self.notify.notify_all();
            }
        }
    }

    /// Pushes an message with prio onto the queue, uses a Stash as temporary storage when the
    /// queue is contended. Drains the stash in the uncontended case.
    pub fn send(&self, entry: K, prio: P, stash: &Stash<K, P>) {
        self.in_progress.fetch_add(1, atomic::Ordering::SeqCst);
        self.is_drained.store(false, atomic::Ordering::SeqCst);
        self.send_with_stash(Message::Msg(entry, prio), stash);
    }

    /// Send the 'Drained' message
    pub(crate) fn send_drained(&self) {
        if self
            .is_drained
            .compare_exchange(
                false,
                true,
                atomic::Ordering::SeqCst,
                atomic::Ordering::SeqCst,
            )
            .is_ok()
        {
            self.in_progress.fetch_add(1, atomic::Ordering::SeqCst);
            self.heap.lock().push(Message::Drained);
            self.notify.notify_one();
        }
    }

    // With lock already hold.
    pub(crate) fn send_drained_with_lock(&self, lock: &mut BinaryHeap<Message<K, P>>) {
        if self
            .is_drained
            .compare_exchange(
                false,
                true,
                atomic::Ordering::SeqCst,
                atomic::Ordering::SeqCst,
            )
            .is_ok()
        {
            self.in_progress.fetch_add(1, atomic::Ordering::SeqCst);
            lock.push(Message::Drained);
            self.notify.notify_one();
        }
    }

    /// Returns the smallest entry from a queue. This entry is wraped in a ReceiveGuard/Message
    pub fn recv_guard(&self) -> ReceiveGuard<K, P> {
        let mut lock = self.heap.lock();
        while lock.is_empty() {
            self.notify.wait(&mut lock);
        }

        let entry = lock.pop().unwrap();

        ReceiveGuard::new(entry, self)
    }

    /// Try to get the smallest entry from a queue. Will return Some<ReceiveGuard> when a
    /// message is available.
    pub fn try_recv_guard(&self) -> Option<ReceiveGuard<K, P>> {
        match self.heap.try_lock() {
            Some(mut queue) => queue.pop().map(|entry| ReceiveGuard::new(entry, self)),
            None => None,
        }
    }

    /// Returns the smallest entry from a queue.
    pub fn recv(&self) -> Message<K, P> {
        let mut lock = self.heap.lock();
        while lock.is_empty() {
            self.notify.wait(&mut lock);
        }

        let msg = lock.pop().unwrap();
        if self.in_progress.fetch_sub(1, atomic::Ordering::SeqCst) == 1 {
            self.send_drained_with_lock(&mut lock);
        }

        msg
    }

    /// Try to get the smallest entry from a queue. Will return Some<Message> when a message
    /// is available.
    pub fn try_recv(&self) -> Option<Message<K, P>> {
        match self.heap.try_lock() {
            Some(mut lock) => {
                let msg = lock.pop().unwrap();
                if self.in_progress.fetch_sub(1, atomic::Ordering::SeqCst) == 1 {
                    self.send_drained_with_lock(&mut lock);
                }
                Some(msg)
            }
            None => None,
        }
    }
}
