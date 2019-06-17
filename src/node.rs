/***************************************************************************************************
Contains everything related to nodes.
The 'Node' trait must be implemented for all nodes that shall participate in the simulation. Currently, the only required function to implement is 'handle_event'.
***************************************************************************************************/

use crate::config::NodeConfig;
use crate::event::{Event, Message, Reception};
use crate::pbft::state::ReplicaState;
use crate::time::Time;
use log::debug;
use std::fmt::Debug;

#[derive(Copy, Clone)]
pub enum NodeType {
    Dummy,
    PBFT,
    Zyzzyva,
    RBFT,
}

/// All nodes need to implement this trait
pub trait Node: Debug {
    /// this function is called from the simulation when an event for the node was in the queue, e.g. a 'reception event' containing a message designated to the node
    fn handle_event(&mut self, reception: Reception, time: Time) -> Option<Vec<Event>>;
}

// Helper function to generate a dynamic node from the given NodeConfig
pub fn build_node(config: NodeConfig) -> Box<dyn Node> {
    match &config.node_type {
        NodeType::Dummy => Box::new(DummyNode::new(config)),
        NodeType::PBFT => Box::new(PBFTNode::new(config)),
        _ => panic!("Only 'dummy' nodes are currently implemented!"),
    }
}

/***************************************************************************************************
I proudly present: on of the dumbest node imaginable, the DummyNode
***************************************************************************************************/

#[derive(Debug)]
pub struct DummyNode {
    id: u32,
}

impl DummyNode {
    pub fn new(config: NodeConfig) -> Self {
        DummyNode { id: config.id }
    }
}

impl Node for DummyNode {
    fn handle_event(&mut self, reception: Reception, time: Time) -> Option<Vec<Event>> {
        debug!(target: "node", "DummyNode is processing a reception: {:?}", &reception);
        let time_current = time;
        let mut return_events = Vec::new();

        if self.id == 1 {
            return_events.push(Event::new_broadcast(
                self.id,
                2,
                Message::Dummy,
                time_current.add_milli(5),
            ));
            return_events.push(Event::new_broadcast(
                self.id,
                2,
                Message::Dummy,
                time_current.add_milli(10),
            ));
        } else if self.id == 2 {
            return_events.push(Event::new_broadcast(
                self.id,
                1,
                Message::Dummy,
                time_current.add_milli(50),
            ));
        }

        Some(return_events)
    }
}

/***************************************************************************************************
PBFT node
Your main playground, i guess
***************************************************************************************************/

/// The `PBFTNode` acts as a host for a single replica. It holds the `ReplicaState`
/// required for the participation in a PBFT cluster.
#[derive(Debug)]
pub struct PBFTNode {
    // if of the node
    id: u32,
    /// holds the state required to take part in a PBFT cluster.
    state: ReplicaState,
}

impl PBFTNode {
    /// Creates a new `PBFTNode` by initializing the `ReplicaState`.
    /// The `ReplicaState` contains the state required for the PBFT operation.
    pub fn new(config: NodeConfig) -> Self {
        PBFTNode {
            state: ReplicaState::new(config.id, config.number_of_nodes),
            id: config.id,
        }
    }
}

impl Node for PBFTNode {
    fn handle_event(&mut self, reception: Reception, time: Time) -> Option<Vec<Event>> {
        debug!(target: "node", "PBFTNode {} is processing a reception: {:?}", self.id ,&reception);

        match reception.message {
            Message::PBFT(pbft_message) => {
                if let Some(out_events) = self.state.handle_message(pbft_message) {

                    let mut events = Vec::<Event>::with_capacity(out_events.len());

                    for (recv_id, msg) in out_events {
                        events.push(Event::new_broadcast(
                            self.id,
                            recv_id,
                            Message::PBFT(msg),
                            time.add_milli(5),
                        ))
                    }

                    return Some(events);
                }
                None
            }
            _ => {
                panic!("Received a non pbft message for a pbft node!");
            }
        }
    }
}