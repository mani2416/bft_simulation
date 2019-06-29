/***************************************************************************************************
Configuration abstractions for the simulation and nodes
Also contains methods called for initialization (ini, log, etc.)
***************************************************************************************************/

use log::{debug, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use mc_utils::ini::env2var;


use crate::node::pbft::messages::{ClientRequest as PBFTCR, PBFTMessage};
use crate::node::zyzzyva::{
    messages::{ClientRequest as ZyzzyvaCR, ZyzzyvaMessage},
    state::CLIENT_ID,
};
use crate::node::NodeType;

use crate::simulation::event::{Event, Message};
use crate::simulation::time::Time;
/// Config to initialize the simulation
pub struct SimulationConfig {
    pub node_type: NodeType,
    pub number_of_nodes: u32,
    next_id: u32,
}

impl SimulationConfig {
    /// Used internally to increment the id counter for each new node
    fn increment_next_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    /// Creates a new NodeConfig
    pub fn create_node_config(&mut self) -> NodeConfig {
        NodeConfig {
            node_type: self.node_type,
            // increment the counter
            id: self.increment_next_id(),
            number_of_nodes: self.number_of_nodes,
        }
    }
}
impl Default for SimulationConfig {
    fn default() -> Self {
        let node_type = env2var::<String>("node.node_type");
        let node_type = match node_type.as_str() {
            "dummy" => NodeType::Dummy,
            "pbft" => NodeType::PBFT,
            "zyzzyva" => NodeType::Zyzzyva,
            "rbft" => NodeType::RBFT,
            _ => panic!(
                "node_type in ini is not available, allowed are 'dummy', 'pbft', 'zyzzyva', 'rbft'"
            ),
        };

        SimulationConfig {
            node_type,
            number_of_nodes: 0,
            next_id: 0,
        }
    }
}
impl SimulationConfig {
    pub fn number_of_nodes(mut self, number_of_nodes: u32) -> SimulationConfig {
        self.number_of_nodes = number_of_nodes;
        self
    }
}

/// Config to initialize a node
pub struct NodeConfig {
    pub node_type: NodeType,
    pub id: u32,
    pub number_of_nodes: u32,
}

/// Config for a batch of requests
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestBatchConfig {
    pub number: u32,
    pub interval: u32,
}

impl RequestBatchConfig {
    pub fn new(number: u32, interval: u32) -> Self {
        RequestBatchConfig { number, interval }
    }

    // create a vector of events, corresponding to the config
    pub fn create_events(
        &self,
        request_id_counter: &mut u64,
        time: Time,
        node_type: NodeType,
    ) -> Vec<Event> {
        let mut result = Vec::with_capacity(self.number as usize);

        for counter in 1..=self.number {
            match node_type {
                NodeType::PBFT => {
                    // the message containing the client request
                    let message = Message::PBFT(PBFTMessage::ClientRequest(PBFTCR {
                        sender_id: 31415,
                        operation: (*request_id_counter as u32),
                    }));
                    //TODO Client requests will go to node '1' by default, add option to define receiver in RequestConfig?
                    let new_time = time.add_milli(u64::from((counter - 1) * self.interval));
                    result.push(Event::new_reception(1, message, new_time));
                }
                NodeType::Zyzzyva => {
                    let message = Message::Zyzzyva(ZyzzyvaMessage::ClientRequest(ZyzzyvaCR {
                        sender_id: 0,
                        operation: (*request_id_counter as u32),
                    }));
                    let new_time = time.add_milli(u64::from((counter - 1) * self.interval));
                    result.push(Event::new_reception(CLIENT_ID, message, new_time));
                }
                _ => panic!(
                    "Received client requests for node type {:?}, which is not implemented yet",
                    node_type
                ),
            }
            *request_id_counter += 1;
        }
        result
    }
}

pub fn log_result(time: Time, node_id: Option<u32>, message: &str) {
    let n: u32 = mc_utils::ini::env2var("node.nodes");

    let mut result = String::new();
    result.push_str(&time.to_string());
    result.push(';');
    if let Some(id) = node_id {
        result.push_str(&id.to_string());
    } else {
        result.push_str("-1");
    }
    result.push(';');
    result.push_str(message);

    debug!(target: &format!("result_{}", n), "{}", &result);
}

/// Read values from the ini and store in environment
pub fn initialize_ini() {
    let ini = mc_utils::ini::get_ini("simulation.ini");
    mc_utils::ini::ini2env("node", "node_type", &ini, None);
    mc_utils::ini::ini2env("node", "nodes_vec", &ini, None);
    mc_utils::ini::ini2env("simulation", "requests", &ini, None);
    mc_utils::ini::ini2env("log", "debug", &ini, None);
    mc_utils::ini::ini2env("log", "result", &ini, None);
    mc_utils::ini::ini2env("network", "omission_probability", &ini, None);
    mc_utils::ini::ini2env("network", "delay_min", &ini, None);
    mc_utils::ini::ini2env("network", "delay_max", &ini, None);
}

/// Initialize the loggers
pub fn initialize_logging() {
    let stdout = ConsoleAppender::builder().build();

    let mut config =
        Config::builder().appender(Appender::builder().build("stdout", Box::new(stdout)));

    if mc_utils::ini::env2var("log.debug") {
        let log_node = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}{n}")))
            .append(false)
            .build("log/debug_nodes.log")
            .unwrap();

        let log_simulation = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}{n}")))
            .append(false)
            .build("log/debug_simulation.log")
            .unwrap();

        config = config
            .appender(Appender::builder().build("log_node", Box::new(log_node)))
            .appender(Appender::builder().build("log_simulation", Box::new(log_simulation)))
            .logger(
                Logger::builder()
                    .appender("log_node")
                    .additive(false)
                    .build("node", LevelFilter::Debug),
            )
            .logger(
                Logger::builder()
                    .appender("log_simulation")
                    .additive(false)
                    .build("simulation", LevelFilter::Debug),
            )
    }

    if mc_utils::ini::env2var("log.result") {
        for n in mc_utils::ini::env2var_vec::<u32>("node.nodes_vec") {
            let r: u32 = mc_utils::ini::env2var("simulation.requests");
            let p: f64 = mc_utils::ini::env2var("network.omission_probability");

            let name_result_logger = format!("result_{}", n);
            let name_result_log_file = format!(
                "log/result_{:0>3}_{:0>3}_{}.log",
                n,
                r,
                (p * 100 as f64) as u32
            );

            let log_result = FileAppender::builder()
                .encoder(Box::new(PatternEncoder::new("{m}{n}")))
                .append(false)
                .build(name_result_log_file.clone())
                .unwrap();

            config = config
                .appender(
                    Appender::builder().build(name_result_log_file.clone(), Box::new(log_result)),
                )
                .logger(
                    Logger::builder()
                        .appender(name_result_log_file)
                        .additive(false)
                        .build(name_result_logger, LevelFilter::Debug),
                )
        }
    }

    let config = config
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();

    log4rs::init_config(config).unwrap();
}
