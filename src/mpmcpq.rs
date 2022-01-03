use std::collections::BinaryHeap;
use std::sync::atomic::{self, AtomicBool, AtomicUsize};

use parking_lot::{Condvar, Mutex};

use crate::*;

/// A queue which orders messages by priority
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
    /// This function waits until the on the queue is locked.
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

        self.notify(notify);
    }

    /// Pushes an message with prio onto the queue, uses a Stash as temporary storage when the
    /// queue is contended. Drains the stash in the uncontended case.
    /// This function does not wait for the lock on the queue.
    pub fn send(&self, entry: K, prio: P, stash: &Stash<K, P>) {
        self.in_progress.fetch_add(1, atomic::Ordering::SeqCst);
        self.is_drained.store(false, atomic::Ordering::SeqCst);

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
            lock.push(Message::Msg(entry, prio));
        } else {
            msgs.push(Message::Msg(entry, prio));
        }

        self.notify(notify);
    }

    /// Pushes a message with prio onto the queue, drains the Stash first.
    /// This function waits until the on the queue is locked.
    pub fn send_sync(&self, entry: K, prio: P, stash: &Stash<K, P>) {
        self.in_progress.fetch_add(1, atomic::Ordering::SeqCst);
        self.is_drained.store(false, atomic::Ordering::SeqCst);

        let notify;
        let mut msgs = stash.msgs.borrow_mut();

        let mut lock = self.heap.lock();
        if msgs.is_empty() {
            notify = Notify::One;
        } else {
            notify = Notify::All;
            msgs.drain(..).for_each(|e| {
                lock.push(e);
            });
        }
        lock.push(Message::Msg(entry, prio));

        self.notify(notify);
    }

    /// Pushes an message to the Stash. will not try to send data to the queue.
    /// Use this to combine some messages together before calling sync() to send them.
    /// This function does not wait for the lock on the queue.
    pub fn send_stash(&self, entry: K, prio: P, stash: &Stash<K, P>) {
        self.in_progress.fetch_add(1, atomic::Ordering::SeqCst);
        self.is_drained.store(false, atomic::Ordering::SeqCst);

        stash.msgs.borrow_mut().push(Message::Msg(entry, prio));
    }

    /// Combines the above to collect at least 'batch_size' messages in the stash before
    /// trying to send them out.  Use this to batch some messages together before calling
    /// sync() to send them.  This function does not wait for the lock on the queue.
    pub fn send_batched(&self, entry: K, prio: P, batch_size: usize, stash: &Stash<K, P>) {
        if stash.len() <= batch_size {
            // append to the stash
            self.send_stash(entry, prio, stash);
        } else {
            // try to send
            self.send(entry, prio, stash);
        }
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
            self.notify(Notify::One);
        }
    }

    /// With lock already hold.
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
            self.notify(Notify::One);
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

    /// Returns the number of messages in flight. This is the .len() plus any receiver that
    /// still holds a guard.  Note: Informal only, this method will be racy when other threads
    /// modify the PriorityQueue.
    pub fn in_progress(&self) -> usize {
        self.in_progress.load(atomic::Ordering::Relaxed)
    }

    /// Returns true when the Stash contains no messages.  Note: Informal only, this method
    /// will be racy when other threads modify the PriorityQueue.
    pub fn is_empty(&self) -> bool {
        self.heap.lock().is_empty()
    }

    /// Returns the number of messages in the stash.  Note: Informal only, this method will be
    /// racy when other threads modify the PriorityQueue.
    pub fn len(&self) -> usize {
        self.heap.lock().len()
    }

    // Note: no capacity(), future versions may use another heap implementation

    /// Reserves capacity for at least `additional` more elements to be inserted in the
    /// PriorityQueue.
    pub fn reserve(&self, additional: usize) {
        self.heap.lock().reserve(additional);
    }

    /// Wakes waiting threads.
    fn notify(&self, notify: Notify) {
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
}
