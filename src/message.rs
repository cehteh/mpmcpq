/// Type for the received message
#[derive(Debug, Clone, Copy)]
pub enum Message<M: Send, P: Ord> {
    /// Message with data M and priority P
    Msg(M, P),
    /// Queue got empty and no other workers processing a ReceiveGuard
    Drained,
    /// Value when taken from a ReceiveGuard, won't be visible to a user.
    Taken,
}

impl<M: Send, P: Ord> Message<M, P> {
    /// Returns a reference to the value of the message.
    pub fn message(&self) -> Option<&M> {
        match &self {
            Message::Msg(msg, _) => Some(msg),
            _ => None,
        }
    }

    /// Returns a reference to the priority of the message.
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

impl<M: Send, P: Ord> Ord for Message<M, P> {
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

impl<M: Send, P: Ord> PartialOrd for Message<M, P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<M: Send, P: Ord> PartialEq for Message<M, P> {
    fn eq(&self, other: &Self) -> bool {
        use Message::*;
        match (self, other) {
            (Msg(_, a), Msg(_, b)) => a == b,
            (Drained, Drained) | (Taken, Taken) => true,
            (_, _) => false,
        }
    }
}

impl<M: Send, P: Ord> Eq for Message<M, P> {}

impl<M: Send, P: Ord> Default for Message<M, P> {
    fn default() -> Self {
        Message::Taken
    }
}
