/// Type for the received message
#[derive(Debug, Clone, Copy)]
pub enum Message<K: Send, P: Ord> {
    /// Message with data K and priority P
    Msg(K, P),
    /// Queue got empty and no other workers processing a ReceiveGuard
    Drained,
    /// Value when taken from a ReceiveGuard, won't be visible to a user.
    Taken,
}

impl<K: Send, P: Ord> Message<K, P> {
    /// Returns a reference to the value of the entry.
    pub fn entry(&self) -> Option<&K> {
        match &self {
            Message::Msg(k, _) => Some(k),
            _ => None,
        }
    }

    /// Returns a reference to the priority of the entry.
    pub fn priority(&self) -> Option<&P> {
        match &self {
            Message::Msg(_, prio) => Some(prio),
            _ => None,
        }
    }

    /// Returns 'true' when the queue is drained
    pub fn is_drained(&self) -> bool {
        matches!(self, Message::Drained)
    }
}

impl<K: Send, P: Ord> Ord for Message<K, P> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            (Message::Msg(_, a), Message::Msg(_, b)) => b.cmp(a),
            (Message::Drained, Message::Drained) => Ordering::Equal,
            (Message::Drained, _) => Ordering::Greater,
            (_, Message::Drained) => Ordering::Less,
            (_, _) => unreachable!("'Taken' should never appear here"),
        }
    }
}

impl<K: Send, P: Ord> PartialOrd for Message<K, P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Send, P: Ord> PartialEq for Message<K, P> {
    fn eq(&self, other: &Self) -> bool {
        use Message::*;
        match (self, other) {
            (Msg(_, a), Msg(_, b)) => a == b,
            (Drained, Drained) | (Taken, Taken) => true,
            (_, _) => false,
        }
    }
}

impl<K: Send, P: Ord> Eq for Message<K, P> {}

impl<K: Send, P: Ord> Default for Message<K, P> {
    fn default() -> Self {
        Message::Taken
    }
}
