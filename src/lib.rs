#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

mod mpmcpq;
pub use self::mpmcpq::PriorityQueue;

mod message;
pub use message::Message;

mod guard;
pub use guard::ReceiveGuard;

mod stash;
pub use stash::Stash;

#[cfg(test)]
mod tests {
    use std::{thread, time};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::io::Write;
    use std::sync::Once;

    #[allow(unused_imports)]
    pub use log::{debug, error, info, trace, warn};
    use env_logger;

    use super::{Message, PriorityQueue, Stash};

    pub fn init_env_logging() {
        static LOGGER: Once = Once::new();
        LOGGER.call_once(|| {
            let counter: AtomicU64 = AtomicU64::new(0);
            let seq_num = move || counter.fetch_add(1, Ordering::Relaxed);

            let start = time::Instant::now();

            env_logger::Builder::from_default_env()
                .format(move |buf, record| {
                    let micros = start.elapsed().as_micros() as u64;
                    writeln!(
                        buf,
                        "{:0>12}: {:0>8}.{:0>6}: {:>5}: {}:{}: {}: {}",
                        seq_num(),
                        micros / 1000000,
                        micros % 1000000,
                        record.level().as_str(),
                        record.file().unwrap_or(""),
                        record.line().unwrap_or(0),
                        thread::current().name().unwrap_or("UNKNOWN"),
                        record.args()
                    )
                })
                .try_init()
                .unwrap();
        });
    }

    #[test]
    fn smoke() {
        init_env_logging();
        let queue: PriorityQueue<String, u64> = PriorityQueue::new();
        let mut stash = Stash::<String, u64>::new(&queue);
        queue.send("test 1".to_string(), 1, &mut stash);
        queue.send("test 3".to_string(), 3, &mut stash);
        queue.send("test 2".to_string(), 2, &mut stash);
        assert_eq!(
            queue.recv_guard().message(),
            &Message::Msg("test 1".to_string(), 1)
        );
        assert_eq!(
            queue.recv_guard().message(),
            &Message::Msg("test 2".to_string(), 2)
        );
        assert_eq!(
            queue.recv_guard().message(),
            &Message::Msg("test 3".to_string(), 3)
        );
        assert_eq!(queue.recv_guard().message(), &Message::Drained);
        assert!(queue.try_recv_guard().is_none());
    }

    #[test]
    fn try_recv() {
        init_env_logging();
        let queue: PriorityQueue<String, u64> = PriorityQueue::new();
        let mut stash = Stash::<String, u64>::new(&queue);
        queue.send("test 1".to_string(), 1, &mut stash);
        queue.send("test 3".to_string(), 3, &mut stash);
        queue.send("test 2".to_string(), 2, &mut stash);
        assert!(queue.try_recv_guard().is_some());
        assert!(queue.try_recv_guard().is_some());
        assert!(queue.try_recv_guard().is_some());
        assert!(queue.try_recv_guard().is_some());
        assert!(queue.try_recv_guard().is_none());
        assert!(queue.try_recv_guard().is_none());
    }

    #[test]
    fn threads() {
        init_env_logging();
        let queue: Arc<PriorityQueue<String, u64>> = Arc::new(PriorityQueue::new());

        let thread1_queue = queue.clone();
        let thread1 = thread::spawn(move || {
            let mut stash1 = Stash::<String, u64>::new(&thread1_queue);
            thread1_queue.send("test 1".to_string(), 1, &mut stash1);
            thread1_queue.send("test 3".to_string(), 3, &mut stash1);
            thread1_queue.send("test 2".to_string(), 2, &mut stash1);
        });
        thread1.join().unwrap();

        let thread2_queue = queue.clone();
        let thread2 = thread::spawn(move || {
            let mut stash2 = Stash::<String, u64>::new(&thread2_queue);
            assert_eq!(
                thread2_queue.recv_guard().message(),
                &Message::Msg("test 1".to_string(), 1)
            );
            assert_eq!(
                thread2_queue.recv_guard().message(),
                &Message::Msg("test 2".to_string(), 2)
            );
            assert_eq!(
                thread2_queue.recv_guard().message(),
                &Message::Msg("test 3".to_string(), 3)
            );
            assert!(thread2_queue.recv_guard().message().is_drained());
            assert!(thread2_queue.try_recv_guard().is_none());
            thread2_queue.send("test 4".to_string(), 4, &mut stash2);
            assert_eq!(
                thread2_queue.recv_guard().message(),
                &Message::Msg("test 4".to_string(), 4)
            );
            assert!(thread2_queue.recv_guard().message().is_drained());
            assert!(thread2_queue.try_recv_guard().is_none());
        });

        thread2.join().unwrap();
    }
}
