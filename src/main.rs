extern crate bft_simulation;

use std::thread;
use std::time::Duration;

use bft_simulation::simulation::config::{
    initialize_ini, initialize_logging, RequestBatchConfig, SimulationConfig,
};
use bft_simulation::simulation::event::{AdminType, EventType};
use bft_simulation::simulation::Simulation;

fn main() {
    // read settings from the ini
    initialize_ini();
    //initialize logger
    initialize_logging();

    // initialize a new simulation
    let config_sim = SimulationConfig::default();
    let mut simulation = Simulation::new(config_sim);

    // get channels to send events to the simulation queue
    let s1 = simulation.get_sender();
    let s2 = simulation.get_sender();

    thread::spawn(move || {
        for _i in 1..2 {
            // add some requests
            s1.send(EventType::Admin(AdminType::ClientRequests(
                RequestBatchConfig::new(10, 1),
            )))
            .unwrap();

            thread::sleep(Duration::from_millis(100));
        }
    });

    // start a new thread to send a cancellation signal after some seconds
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(3));
        s2.send(EventType::Admin(AdminType::Stop)).unwrap();
    });

    // start the simulation
    //    thread::sleep(Duration::from_millis(100));
    simulation.start_handling();
}
