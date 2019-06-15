extern crate bft_simulation;

use std::thread;
use std::time::Duration;

use bft_simulation::config::{initialize_ini, initialize_logging, SimulationConfig};
use bft_simulation::event::{Event, Message};
use bft_simulation::pbft::messages::{ClientRequest, PBFTMessage};
use bft_simulation::simulation::Simulation;
use bft_simulation::time::Time;

fn main() {

    // read settings from the ini
    initialize_ini();
    //initialize logger
    initialize_logging();

    // initialize a new simulation
    let config_sim = SimulationConfig::new();
    let mut simulation = Simulation::new(config_sim);

    // get channels to send events to the simulation queue
    let s1 = simulation.get_sender();
    let s2 = simulation.get_sender();

    thread::spawn(move || {
        for i in 1..101 {
            thread::sleep(Duration::from_millis(50));

            let request = Message::PBFT(PBFTMessage::ClientRequest(ClientRequest {
                sender_id: 31415,
                operation: i,
            }));
            // inject an artificial node reception event in the queue to get things started
            s1.send(Event::new_reception(1, request, Time::new(50 * i as u64)))
                .unwrap();
        }
    });

    // start a new thread to send a cancellation signal after some seconds
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(10));
        s2.send(Event::new_admin()).unwrap();
    });

    // start the simulation
    simulation.start_handling();
}
