extern crate bft_simulation;

use std::thread;
use std::time::Duration;

use bft_simulation::config::{SimulationConfig, initialize_ini, initialize_logging};
use bft_simulation::event::{Event, Message};
use bft_simulation::time::Time;
use bft_simulation::simulation::Simulation;

fn main() {

    // read settings from the ini
    initialize_ini();
    //initialize logger
    initialize_logging();

    // initialize a new simulation
    let config_sim = SimulationConfig::new();
    let mut simulation = Simulation::new(config_sim);

    // get a channel to send events to the simulation queue
    let sender = simulation.get_sender();

    // inject an artificial node reception event in the queue to get things started
    sender.send(
        Event::new_reception(1, Message::Dummy, Time::new(10))
    ).unwrap();

    // start a new thread to send a cancellation signal after some seconds
    thread::spawn(move||{
        thread::sleep(Duration::from_secs(5));
        sender.send(Event::new_admin()).unwrap();
    });

    // start the simulation
    simulation.start_handling();
}
