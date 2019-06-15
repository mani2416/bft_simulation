/***************************************************************************************************
Core of the simulation based on an event queue
***************************************************************************************************/


use crate::config::SimulationConfig;
use crate::event::{Event, EventType};
use crate::network::Network;
use crate::node::{build_node, Node};
use crate::time::Time;
use log::{debug, info, warn};
use std::collections::{binary_heap::BinaryHeap, HashMap};
use std::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

/// Simulation abstraction, based on an event queue
#[derive(Debug)]
pub struct Simulation {
    // Queue for all events
    event_queue: Arc<Mutex<BinaryHeap<Event>>>,
    // Map with nodes, referenced by id
    node_map: HashMap<u32, Box<dyn Node>>,
    // Network abstraction
    network: Network,
    // Global simulation time, update with each received event
    time: Time,
    // This channel is a preparation so we can feed events to the simulation from an external source, e.g. administrative
    event_sender: Sender<Event>,
}

impl Simulation {
    pub fn new(mut config: SimulationConfig) -> Self {
        // initialize a channel so we can interact with the simulation
        let (event_sender, event_receiver) = mpsc::channel();
        // binary heap, so all events are automatically ordered according to their time
        let event_queue = Arc::new(Mutex::new(BinaryHeap::new()));
        // Create the nodes and store in a hash map
        let mut node_map = HashMap::with_capacity(config.number_of_nodes as usize);

        for n in 1..config.number_of_nodes + 1 {
            node_map.insert(n, build_node(config.create_node_config()));
        }

        let result = Simulation {
            node_map,
            event_queue,
            event_sender,
            network: Network::new(),
            time: Time::new(0),
        };

        // start receiving on the channel
        result.start_receiving(event_receiver);

        result

    }

    // Starts the action: loops over events in the queue and executes them sequentially
    pub fn start_handling(&mut self) {
        info!("Simulation started");

        loop {
            // access the quue, get the latest element and free the mutex
            let mut queue = self.event_queue.lock().expect("Mutex lock poisoned. It appears that someone panicked, that wasn't allowed to panic");
            let event = (*queue).pop();
            drop(queue);

            // if an event was returned, handle it
            if let Some(event) = event {
                debug!(target: "simulation", "Processing event: {:?}", &event);
                match event.event_type {
                    EventType::Admin => {
                        info!("Received admin event, stopping simulation!");
                        break;
                    }
                    EventType::Network => {
                        warn!(target: "simulation", "Network event still unimplemented")
                    }
                    EventType::Reception(r) => {
                        self.update_time(event.time);
                        let receiver = self.node_map.get_mut(&r.id).expect(&format!(
                            "A message was sent to a non-existent node id {}",
                            &r.id
                        ));
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
                }
                thread::sleep(Duration::from_millis(0));
            }

        }
    }

    fn update_time(&mut self, time: Time) {
        // logically it would have to be "<", but time was rewritten to be sorted reverse, so we check for the new time to be "smaller", i.e. after the current time
        if time > self.time {
            panic!("The simulation handled an event that was before it's current time!");
        }
        self.time = time;
    }

    fn add_event_to_queue(&self, event: Event) {
        let mut queue = self.event_queue.lock().expect(
            "Mutex lock poisoned. It appears that someone panicked, that wasn't allowed to panic",
        );
        (*queue).push(event);
    }

    fn add_events_to_queue(&self, events: Vec<Event>) {
        let mut queue = self.event_queue.lock().expect(
            "Mutex lock poisoned. It appears that someone panicked, that wasn't allowed to panic",
        );
        for event in events {
            (*queue).push(event);
        }
    }

    /// Return a sender to the event_queue for this handler
    pub fn get_sender(&self) -> Sender<Event> {
        self.event_sender.clone()
    }

    /// Starts the listener thread
    fn start_receiving(&self, receiver: Receiver<Event>) {
        let queue = Arc::clone(&self.event_queue);
        thread::spawn(move || loop {
            if let Ok(received_event) = receiver.recv() {
                debug!(target: "simulation", "Received an event through the channel: {:?}", &received_event);
                let mut queue = queue.lock().expect("Mutex lock poisoned. It appears that someone panicked, that wasn't allowed to panic");
                (*queue).push(received_event);
            }
        });
    }
}