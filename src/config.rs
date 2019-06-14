/***************************************************************************************************
Configuration abstractions for the simulation and nodes
***************************************************************************************************/

use crate::node::NodeType;
use mc_utils::ini::env2var;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Config, Appender, Root, Logger};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;

/// Config tp initialize the simulation
pub struct SimulationConfig{
    pub node_type : NodeType,
    pub number_of_nodes : u32,
    next_id : u32,
}

impl SimulationConfig{

    /// Create config
    pub fn new() -> Self{

        // read values from ini
        let node_type = env2var::<String>("node.node_type");
        let node_type = match node_type.as_str(){
            "dummy" => NodeType::Dummy,
            "pbft" => NodeType::PBFT,
            "zyzzyva" => NodeType::Zyzzyva,
            "rbft" => NodeType::RBFT,
            _ => panic!("node_type in ini is not available, allowed are 'dummy', 'pbft', 'zyzzyva', 'rbft'"),
        };
        let number_of_nodes = env2var("node.number_of_nodes");

        SimulationConfig{
            node_type,
            number_of_nodes,
            next_id : 0,
        }
    }

    /// Used internally to increment the id counter for each new node
    fn increment_next_id(&mut self) -> u32{
        self.next_id += 1;
        self.next_id
    }

    /// Creates a new NodeConfig
    pub fn create_node_config(&mut self) -> NodeConfig{
        NodeConfig{
            node_type : self.node_type,
            // increment the counter
            id : self.increment_next_id(),
            number_of_nodes : self.number_of_nodes,
        }
    }
}

/// Config to initialize a single node
pub struct NodeConfig{
    pub node_type : NodeType,
    pub id: u32,
    pub number_of_nodes : u32,
}

pub fn initialize_ini(){
    let ini = mc_utils::ini::get_ini("simulation.ini");
    mc_utils::ini::ini2env("node", "node_type", &ini, None);
    mc_utils::ini::ini2env("node", "number_of_nodes", &ini, None);
    mc_utils::ini::ini2env("log", "debug", &ini, None);
}

pub fn initialize_logging(){

    let stdout = ConsoleAppender::builder().build();

    let log_node = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}{n}")))
        .append(false)
        .build("log/nodes.log").unwrap();

    let log_simulation = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}{n}")))
        .append(false)
        .build("log/simulation.log").unwrap();

    let mut config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)));

    if mc_utils::ini::env2var("log.debug") {
        config = config
        .appender(Appender::builder().build("log_node", Box::new(log_node)))
        .appender(Appender::builder().build("log_simulation", Box::new(log_simulation)))
        .logger(Logger::builder()
            .appender("log_node")
            .additive(false)
            .build("node", LevelFilter::Debug))
        .logger(Logger::builder()
            .appender("log_simulation")
            .additive(false)
            .build("simulation", LevelFilter::Debug))
    }

    let config = config
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();

    log4rs::init_config(config).unwrap();
}