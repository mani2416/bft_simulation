extern crate binary_heap_plus;
extern crate tokio;
extern crate tokio_threadpool;
extern crate rand;
extern crate mc_utils;
extern crate log;
extern crate log4rs;

pub mod node;
pub mod event;
pub mod time;
pub mod config;
pub mod network;
pub mod simulation;

#[cfg(test)]
mod tests {
    use crate::time::Time;
    use crate::event::{Event, Message};

    #[test]
    /// Check the ordering of time (lower time must be greater, so the heap removes it first)
    fn check_time_ordering() {
        let time_small = Time::new(1);
        let time_large = Time::new(100);
        assert!(time_small > time_large);
    }

    #[test]
    /// Check the ordering of Events (admin and time ordering)
    fn check_event_ordering() {
        let event_early = Event::new_broadcast(1, 2, Message::Dummy, Time::new(1));
        let event_late = Event::new_broadcast(1, 2, Message::Dummy, Time::new(100));
        let event_admin = Event::new_admin();
        assert!(event_early > event_late);
        assert!(event_admin > event_early);
    }
}
