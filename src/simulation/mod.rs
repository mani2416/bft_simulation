use std::collections::{binary_heap::BinaryHeap, HashMap};
use std::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

use log::{debug, info, warn};

use config::SimulationConfig;
use event::{AdminType, Event, EventType};


use crate::network::Network;
use crate::node::{build_node, Node, NodeType};
use crate::simulation::config::log_result;
use mc_utils::ini::env2var;
use time::Time;
pub mod config;
pub mod event;
pub mod time;

/***************************************************************************************************
Core of the simulation based on an event queue
***************************************************************************************************/

/// Simulation abstraction, based on an event queue
#[derive(Debug)]
pub struct Simulation {
    // Queue for all events
    event_queue: Arc<Mutex<BinaryHeap<Event>>>,
    // Map with nodes, referenced by id
    node_map: HashMap<u32, Box<dyn Node>>,
    // Type of nodes in the current simulation
    node_type: NodeType,
    // Network abstraction
    network: Network,
    // Global simulation time, update with each received event
    time: Time,
    // This channel is a preparation so we can feed events to the simulation from an external source, e.g. administrative
    external_sender: Sender<EventType>,
    // Request counter
    request_counter: u64,
}

impl Simulation {
    pub fn new(mut config: SimulationConfig) -> Self {
        // initialize a channel so we can interact with the simulation
        let (external_sender, external_receiver) = mpsc::channel();
        // binary heap, so all events are automatically ordered according to their time
        let event_queue = Arc::new(Mutex::new(BinaryHeap::new()));
        // Create the nodes and store in a hash map
        let mut node_map = HashMap::with_capacity(config.number_of_nodes as usize);

        for n in 1..=config.number_of_nodes {
            node_map.insert(n, build_node(config.create_node_config()));
        }

        let result = Simulation {
            node_map,
            node_type: config.node_type,
            event_queue,
            external_sender,
            network: Network::new(),
            time: Time::new(0),
            request_counter: 1,
        };

        // start receiving on the channel
        result.start_receiving(external_receiver);
        result
    }

    // Starts the action: loops over events in the queue and executes them sequentially
    pub fn start_handling(&mut self) {
        info!(
            "Simulation started for n = {} of type {:?}",
            self.node_map.len(),
            self.node_type
        );

        let mut timeout_active: Option<Instant> = None;

        loop {
            // access the queue, get the latest element and free the mutex
            let mut queue = self.event_queue.lock().expect("Mutex lock poisoned. It appears that someone panicked, that wasn't allowed to panic");
            let event = (*queue).pop();
            drop(queue);

            // if an event was returned, handle it
            if let Some(event) = event {
                debug!(target: "simulation", "Processing event: {:?}", &event);

                if timeout_active.is_some() {
                    timeout_active = None;
                }

                match event.event_type {
                    EventType::Admin(admin_type) => match admin_type {
                        AdminType::Stop => {
                            info!("Received admin event, stopping simulation!");
                            log_result(self.time, None, "Simulation finished");
                            break;
                        }
                        AdminType::ClientRequests(config) => {
                            let new_events = config.create_events(
                                &mut self.request_counter,
                                self.time,
                                self.node_type,
                            );
                            for event in new_events {
                                self.add_event_to_queue(event);
                            }
                        }
                    },
                    EventType::Network => {
                        warn!(target: "simulation", "Network event still unimplemented")
                    }
                    EventType::Reception(r) => {
                        self.update_time(event.time);
                        let receiver = self.node_map.get_mut(&r.id).unwrap_or_else(|| {
                            panic!("A message was sent to a non-existent node id {}", &r.id)
                        });
                        if let Some(new_events) = (**receiver).handle_event(r, self.time) {
                            self.add_events_to_queue(new_events);
                        }
                    }
                    EventType::Broadcast(b) => {
                        self.update_time(event.time);
                        if let Some(r) = self.network.handle_broadcast(self.time, b) {
                            self.add_event_to_queue(r);
                        }
                    }
                    EventType::Timeout(t) => {
                        self.update_time(event.time);
                        let timeout = env2var::<u64>("node.client_timeout");
                        let time = self.time.add_milli(timeout);
                        let event = Event::new_reception(t.c_id, t.message, time);

                        self.add_event_to_queue(event);
                    }

                }
            } else {
                if let Some(time) = timeout_active {
                    if Instant::now().duration_since(time) > Duration::from_secs(1) {
                        // Well, this is a little with the shotgun through the knee to hit the eye. nut iit should do the job:
                        // We inform our external receiver to stop the simulation, which should stop this loop
                        info!("Simulation queue timed out, sending termination signal");
                        self.external_sender
                            .send(EventType::Admin(AdminType::Stop))
                            .unwrap();
                        // Reset the timeout
                        timeout_active = Some(Instant::now());
                    }
                    // Wait some time
                    thread::sleep(Duration::from_millis(500));
                } else {
                    timeout_active = Some(Instant::now());
                }
            }
        }
    }

    fn update_time(&mut self, time: Time) {
        // logically, it would have to be "<", but time was rewritten to be sorted reverse, so we check for the new time to be "smaller", i.e. after the current time
        if time > self.time {
            panic!("The simulation handled an event that was before its current time!");
        }
        self.time = time;
    }

    fn add_event_to_queue(&self, event: Event) {
        let mut queue = self.event_queue.lock().expect(
            "Mutex lock poisoned. It appears that someone panicked, that wasn't allowed to panic",
        );
        debug!(target: "simulation", "Adding event to queue: {:?}", &event);
        (*queue).push(event);
    }

    fn add_events_to_queue(&self, events: Vec<Event>) {
        for event in events {
            self.add_event_to_queue(event);
        }
    }

    /// Return a sender to the event_queue for this handler
    pub fn get_sender(&self) -> Sender<EventType> {
        self.external_sender.clone()
    }

    /// Starts the listener thread
    fn start_receiving(&self, receiver: Receiver<EventType>) {
        let queue_clone = Arc::clone(&self.event_queue);

        debug!(target: "simulation", "Receiver thread: Starting");
        thread::spawn(move || loop {
            if let Ok(event_type) = receiver.recv() {
                debug!(target: "simulation", "Receiver thread: Received event type: {:?}", &event_type);
                let mut queue = queue_clone.lock().expect("Mutex lock on queue poisoned. It appears that someone panicked, that wasn't allowed to panic.");
                match event_type {
                    EventType::Admin(admin_type) => {
                        match admin_type{
                            AdminType::Stop => {
                                (*queue).push(Event::new_admin_stop());
                                debug!(target: "simulation", "Receiver thread: Terminating");
                                break;
                            },
                            AdminType::ClientRequests(config) => (*queue).push(Event::new_admin_requests_from_config(config)),
                        }
                    },
                    _ => panic!(" Receiver thread: Received '{:?}' from external channel, but only Admin events are configured to be arrive from an external channel", event_type)
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::simulation::event::{Event, Message};
    use crate::simulation::time::Time;

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
        let event_admin = Event::new_admin_stop();
        assert!(event_early > event_late);
        assert!(event_admin > event_early);
    }
}
