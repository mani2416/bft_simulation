/***************************************************************************************************
Everything related to the network.
***************************************************************************************************/

use log::debug;
use mc_utils::ini::env2var;
use rand::Rng;
use rand::rngs::ThreadRng;

use crate::simulation::event::{Broadcast, Event};
use crate::simulation::time::Time;

/// Network abstraction
#[derive(Debug, Default)]
pub struct Network {
    omission_prob: f64,
    delay_min: u32,
    delay_max: u32,
    my_rng: ThreadRng,
}
impl Network {
    pub fn new() -> Self {
        Network {
            omission_prob: env2var("network.omission_probability"),
            delay_min: env2var("network.delay_min"),
            delay_max: env2var("network.delay_max"),
            my_rng: rand::thread_rng(),
        }
    }

    /// Handles broadcasts on the network
    pub fn handle_broadcast(&mut self, time: Time, broadcast: Broadcast) -> Option<Event> {
        // apply the omission probability
        if broadcast.can_omit
            && self.omission_prob > 0.0
            && self.my_rng.gen::<f64>() <= self.omission_prob
        {
            debug!(target: "simulation", "Message is omitted: {:?}", &broadcast);
            return None;
        }

        // set the delay to random value between the min and max value
        let delay = if self.delay_min == self.delay_max {
            u64::from(self.delay_min)
        } else {
            self.my_rng
                .gen_range(u64::from(self.delay_min), u64::from(self.delay_max))
        };

        // Create the respective reception event
        Some(Event::new_reception(
            broadcast.id_to,
            broadcast.message,
            time.add_milli(delay),
        ))
    }
}
