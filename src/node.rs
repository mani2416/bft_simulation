/***************************************************************************************************
Contains everything related to nodes.
The 'Node' trait must be implemented for all nodes that shall participate in the simulation. Currently, the only required function to implement is 'handle_event'.
***************************************************************************************************/

use super::event::Event;
use crate::config::NodeConfig;
use crate::event::{Message, Reception};
use crate::time::Time;
use std::fmt::Debug;
use log::debug;

/// The allowed types of Nodes
#[derive(Copy, Clone)]
pub enum NodeType{
    Dummy, PBFT, Zyzzyva, RBFT,
}

/// All nodes need to implement this trait
pub trait Node : Debug{
    /// this function is called from the simulation when an event for the node was in the queue, e.g. a 'reception event' containing a message designated to the node
    fn handle_event(&mut self, reception: Reception, time: Time) -> Option<Vec<Event>>;
}

// Helper function to generate a dynamic node from the given NodeConfig
pub fn build_node(config: NodeConfig) -> Box<dyn Node> {
    match &config.node_type{
        NodeType::Dummy => return Box::new(DummyNode::new(config)),
        _ => panic!("Only 'dummy' nodes are currently implemented!"),
    }
}

/***************************************************************************************************
I proudly present: on of the dumbest node imaginable, the DummyNode
***************************************************************************************************/

#[derive(Debug)]
pub struct DummyNode{
    id : u32,
}

impl DummyNode{
    pub fn new(config: NodeConfig) -> Self {
        DummyNode {
            id: config.id,
        }
    }
}

impl Node for DummyNode {

    fn handle_event(&mut self, reception: Reception, time: Time) -> Option<Vec<Event>>{

        debug!(target: "node", "DummyNode is processing a reception: {:?}", &reception);
        let time_current = time;
        let mut return_events= Vec::new();

        if self.id == 1{
            return_events.push(Event::new_broadcast(self.id,2, Message::Dummy, time_current.add_milli(5)));
            return_events.push(Event::new_broadcast(self.id,2, Message::Dummy, time_current.add_milli(10)));
        }else if self.id == 2 {
            return_events.push(Event::new_broadcast(self.id, 1, Message::Dummy, time_current.add_milli(50)));
        }

        Some(return_events)
    }
}

/***************************************************************************************************
PBFT node
Your main playground, i guess
***************************************************************************************************/

#[derive(Debug)]
pub struct PBFTNode{
//    id : u32,
//    number_of_nodes: u32,
}

impl Node for PBFTNode {
    fn handle_event(&mut self, _reception: Reception, _time: Time) -> Option<Vec<Event>>{
        //todo lots ;-)
        None
    }
}