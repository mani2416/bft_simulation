/***************************************************************************************************
Everything related to events.
***************************************************************************************************/

use crate::time::Time;
use std::cmp::Ordering;

/// The types of events that can happen in the simulation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventType{
    Admin, Network, Broadcast(Broadcast), Reception(Reception),
}

// An event abstraction, contains the time of the event and the event_type
#[derive(Debug, Eq, PartialEq, PartialOrd)]
pub struct Event {
    pub time: Time,
    pub event_type : EventType,
}

impl Event {
    fn new(event_type : EventType, time: Time) -> Self{
        Event {
            event_type,
            time: time,
        }
    }

    /// To generate a new admin event
    pub fn new_admin() -> Self{
        Event::new(EventType::Admin, Time::new(0))
    }

    /// To generate a new broadcast event
    pub fn new_broadcast(id_from: u32, id_to: u32, message: Message, time: Time) -> Self{
        Event::new(EventType::Broadcast(Broadcast::new(id_from, id_to, message)), time)
    }

    /// To generate a new reception event
    pub fn new_reception(id: u32, message: Message, time: Time) -> Self{
        Event::new(EventType::Reception(Reception::new(id, message)), time)
    }
}

// Order the events according to 'Time', with Admin events always having priority
impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.event_type{
            EventType::Admin => Ordering::Greater,
            _ => {
                let result = self.time.cmp(&other.time);
                result
            }
        }
    }
}

/// Broadcast abstraction, is part of the EventType
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Broadcast{
    pub id_from : u32,
    pub id_to : u32,
    pub message : Message
}
impl Broadcast{
    pub fn new(id_from: u32, id_to: u32, message: Message) -> Self{
        Broadcast{
            id_from,
            id_to,
            message,
        }
    }
}

/// Reception abstraction, is part of the EventType
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reception{
    pub id : u32,
    pub message : Message
}
impl Reception{
    pub fn new(id: u32, message: Message) -> Self{
        Reception{
            id,
            message,
        }
    }
}

/// Message abstraction
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Message{
    Dummy, PBFT(PBFTMessage), //Zyzzyva(ZyzzyvaMessage), RBFT(RBFTMessage),
}

/// PBFT Message
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PBFTMessage{
    //todo whatever you require
}
