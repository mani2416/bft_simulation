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

    let node_vec = mc_utils::ini::env2var_vec::<u32>("node.nodes_vec");
    for n in node_vec {

        mc_utils::ini::env::set_var("node.nodes", n.to_string());

        // initialize a new simulation
        let config_sim = SimulationConfig::default();
        let mut simulation = Simulation::new(config_sim.number_of_nodes(n));

        // get channels to send events to the simulation queue
        let s1 = simulation.get_sender();

        thread::spawn(move || {
            for _i in 1..2 {
                // add some requests
                s1.send(EventType::Admin(AdminType::ClientRequests(
                    RequestBatchConfig::new(mc_utils::ini::env2var("simulation.requests"), 1000),
                )))
                    .unwrap();

                thread::sleep(Duration::from_millis(100));
            }
        });

        simulation.start_handling();
    }
}
