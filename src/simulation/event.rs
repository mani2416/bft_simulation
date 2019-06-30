/***************************************************************************************************
Everything related to events.
***************************************************************************************************/

use std::cmp::Ordering;

use crate::node::pbft::messages::PBFTMessage;
use crate::node::zyzzyva::messages::ZyzzyvaMessage;
use crate::simulation::config::RequestBatchConfig;
use crate::simulation::time::Time;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdminType {
    ClientRequests(RequestBatchConfig),
    Stop,
}

/// The types of events that can happen in the simulation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventType {
    Admin(AdminType),
    Network,
    Broadcast(Broadcast),
    Reception(Reception),
    Timeout(Timeout)
}

// An event abstraction, contains the time of the event and the event_type
#[derive(Debug, Eq, PartialEq, PartialOrd)]
pub struct Event {
    pub time: Time,
    pub event_type: EventType,
}

impl Event {
    fn new(event_type: EventType, time: Time) -> Self {
        Event { event_type, time }
    }

    /// To generate a new admin event
    pub fn new_admin_stop() -> Self {
        Event::new(EventType::Admin(AdminType::Stop), Time::new(0))
    }

    // Generate a batch of requests
    pub fn new_admin_requests(number: u32, interval: u32) -> Self {
        Event::new(
            EventType::Admin(AdminType::ClientRequests(RequestBatchConfig::new(
                number, interval,
            ))),
            Time::new(0),
        )
    }

    pub fn new_admin_requests_from_config(config: RequestBatchConfig) -> Self {
        Event::new(
            EventType::Admin(AdminType::ClientRequests(config)),
            Time::new(0),
        )
    }

    /// To generate a new broadcast event
    pub fn new_broadcast(id_from: u32, id_to: u32, message: Message, time: Time) -> Self {
        Event::new(
            EventType::Broadcast(Broadcast::new(id_from, id_to, message)),
            time,
        )
    }

    /// To generate a new reception event
    pub fn new_reception(id: u32, message: Message, time: Time) -> Self {
        Event::new(EventType::Reception(Reception::new(id, message)), time)
    }

    pub fn new_timeout(c_id: u32, message: Message, time: Time) -> Self {
        Event::new(EventType::Timeout(Timeout::new(c_id, message)), time)
    }
}

// Order the events according to 'Time', with Admin events always having priority
impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.event_type {
            EventType::Admin(_) => Ordering::Greater,
            _ => self.time.cmp(&other.time),
        }
    }
}

/// Broadcast abstraction, is part of the EventType
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Broadcast {
    pub id_from: u32,
    pub id_to: u32,
    pub message: Message,
}
impl Broadcast {
    pub fn new(id_from: u32, id_to: u32, message: Message) -> Self {
        Broadcast {
            id_from,
            id_to,
            message,
        }
    }
}

/// Reception abstraction, is part of the EventType
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reception {
    pub id: u32,
    pub message: Message,
}
impl Reception {
    pub fn new(id: u32, message: Message) -> Self {
        Reception { id, message }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timeout {
    pub c_id: u32,
    pub message: Message
}
impl Timeout {
    pub fn new(c_id: u32, message: Message) -> Self {
        Timeout { c_id, message } 
    }
}

/// Message abstraction
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Message {
    Dummy,
    PBFT(PBFTMessage), 
    Zyzzyva(ZyzzyvaMessage), 
    //RBFT(RBFTMessage),
}
