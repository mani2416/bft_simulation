/***************************************************************************************************
Everything erlated to the network.
***************************************************************************************************/

use crate::event::{Broadcast, Event};
use crate::time::Time;

/// Network abstraction
#[derive(Debug)]
pub struct Network {}
impl Network {
    pub fn new() -> Self {
        Network {}
    }

    /// Handles broadcasts on the network
    pub fn handle_broadcast(&self, time: Time, broadcast: Broadcast) -> Option<Event> {
        //todo currently no message losses, no delays
        Some(Event::new_reception(
            broadcast.id_to,
            broadcast.message,
            time.add_milli(0),
        ))
    }

}