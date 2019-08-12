/***************************************************************************************************
Everything related to time.
***************************************************************************************************/

use std::cmp::{Ord, Ordering};
use std::ops::{Add, Sub};

/// The time abstraction used in the simulation.
/// This struct is used as the sorting parameter for the events in the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time {
    milli_seconds: u64,
}

impl Time {
    pub fn new(milli_seconds: u64) -> Self {
        Time { milli_seconds }
    }

    pub fn add_milli(self, milli: u64) -> Time {
        Time {
            milli_seconds: self.milli_seconds + milli,
        }
    }

    pub fn sub_milli(self, milli: u64) -> Time {
        Time {
            milli_seconds: self.milli_seconds - milli,
        }
    }

    pub fn milli(&self) -> u64 {
        self.milli_seconds
    }
}

impl Sub for Time {
    type Output = Time;

    fn sub(self, other: Time) -> Time {
        Time {
            milli_seconds: self.milli_seconds - other.milli_seconds,
        }
    }
}

impl Add for Time {
    type Output = Time;

    fn add(self, other: Time) -> Time {
        Time {
            milli_seconds: self.milli_seconds + other.milli_seconds,
        }
    }
}

// We have to reverse the ordering, because the binary tree sorts with max first
impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.milli_seconds.ge(&other.milli_seconds) {
            Ordering::Less
        } else if self.milli_seconds.le(&other.milli_seconds) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

// I have no idea why i can't derive PartialOrd, but the tests fail if i do. So here it is
impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ToString for Time {
    fn to_string(&self) -> String {
        self.milli_seconds.to_string()
    }
}
