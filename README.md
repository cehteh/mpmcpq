# Description

A multi-producer multi-consumer priority queue with some special features.

Allows threads to send and receive messages ordered by priority.

While a thread is processing a message, received messages can be protected by a guard this
keeps the queue in a non-drained state the thread may send new messages to the queue to be
processed. Once the queue becomes empty and all guards are dropped a single 'Drained' message
is generated to notify (one of) the receivers that no more data is to be expected.

When performance is more important than exact priority order threads can hold a 'stash' to
temporary store messages to be send. When the queue is contended messages will be put on this
stash instead. Once the thread gets the lock on the queue the stash will be moved over to the
queue.
