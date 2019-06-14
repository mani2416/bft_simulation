//! The log module contains initialization method for logging and two special logging macros: mentor! and mentini!

extern crate chrono;
extern crate log;
extern crate simplelog;

use self::chrono::{DateTime, Utc};
use self::log::debug;
use self::simplelog::*;
use super::filehelper::FileHelper;
use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::str;

/// The &str arguments define the name of the logging files for debug, info and error. max_minutes_last_log specifies after which time (since the last execution), the WON'T be appended
///
/// The following example will allow appending, if the last execution was within five minutes
/// # Examples
/// ```
/// use mc_log::initialize_logger;
///
/// initialize_logger(Some("debug.log"), None, Some("error.log"), Some(5))
/// ```
pub fn initialize_logger(
    file_name_error: Option<&str>,
    file_name_info: Option<&str>,
    file_name_debug: Option<&str>,
    max_minutes_last_log: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let mut append = max_minutes_last_log != Some(0);

    let file_name_most_verbose = file_name_debug
        .unwrap_or_else(|| file_name_info.unwrap_or_else(|| file_name_error.unwrap()));

    let mut message = String::new();

    if append && max_minutes_last_log != None {
        // try to open the file
        let content = FileHelper::read_file_to_string(file_name_most_verbose)?;
        //parse last-log-time and set append accordingly
        append = check_for_last_hook_in_file(&content, max_minutes_last_log.unwrap(), &mut message)
    }

    // initialize all loggers
    let mut loggers: Vec<Box<SharedLogger>> = Vec::new();
    if file_name_debug.is_some() {
        loggers.push(create_log(
            file_name_debug.unwrap(),
            LevelFilter::Debug,
            append,
        ));
    }
    if file_name_info.is_some() {
        loggers.push(create_log(
            file_name_info.unwrap(),
            LevelFilter::Info,
            append,
        ));
    }
    if file_name_error.is_some() {
        loggers.push(create_log(
            file_name_error.unwrap(),
            LevelFilter::Error,
            append,
        ));
    }
    CombinedLogger::init(loggers).unwrap();

    // feedback
    message.push_str(&format!("logger_start_hook: {:?}", Utc::now().to_rfc3339()));
    debug!("{}", message);
    Ok(())
}

// helper method to find the last time hook in the log
fn check_for_last_hook_in_file(to_parse: &str, max_minutes: usize, message: &mut String) -> bool {
    let idx = match to_parse.rfind("logger_start_hook") {
        Some(i) => i,
        None => {
            message.push_str("No 'last-log-time' in log file, only appending to file");
            return false;
        }
    };

    let temp: Vec<&str> = to_parse.get(idx..).unwrap().split('\"').collect();
    let last = match DateTime::parse_from_rfc3339(temp[1]) {
        //convert the time and compare to given threshold
        Ok(dt) => dt,
        Err(err) => {
            message.push_str(&format!(
                "Failed to convert time string from log\nError: {}",
                err
            ));
            return false;
        }
    };

    let now: DateTime<Utc> = Utc::now();
    let since = now.signed_duration_since(last);
    if (since.num_minutes() as usize) < max_minutes {
        return false;
    }

    message.push_str(&format!(
        "Last log time is older than {} minutes, truncating log",
        max_minutes
    ));
    true
}

// creates a single logging instance
fn create_log(file_name: &str, level_filter: LevelFilter, append: bool) -> Box<SharedLogger> {
    if !append {
        let _ = fs::remove_file(file_name);
    }
    WriteLogger::new(
        level_filter,
        Config {
            time: Some(Level::Error),
            level: Some(Level::Error),
            target: Some(Level::Error),
            location: Some(Level::Trace),
            ..Default::default()
        },
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_name)
            .unwrap(),
    )
}

/// mentoring functions, methods, nested methods also arrayelements
/// unexplored chaing of calls, parents are liabble/responsible for their childrens
#[macro_export]
macro_rules! mentor (
    // methods. nested methods also arrayelements
    ($($i: ident $([$ind: expr])*).+ ; $($params:expr),*) => {
        {
            let name = stringify!($($i$([$ind])*).+ ).to_string().replace(" ", "");
            debug!("Starting {}", name);
            let x = $($i$([$ind])*).+($($params),*);
            debug!("Finished {}", name);
            x
        }
    };
    // methods. nested methods also arrayelements with label
    ($n:expr, $($i: ident $([$ind: expr])*).+ ; $($params:expr),*) => {
        {
            debug!("Starting {}", $n);
            let x = $($i$([$ind])*).+($($params),*);
            debug!("Finished {}", $n);
            x
        }
    };
    // functions and static methods
    ($f: expr; $($params:expr),*) => {
        {
            debug!("Starting {}", stringify!($f));
            let x = $f($($params),*);
            debug!("Finished {}",stringify!($f));
            x
        }
    };
    // functions and static methods with label
    ($n:expr,  $f:expr; $($params:expr),*) => {
        {
            debug!("Starting {}", $n);
            let x = $f($($params),*);
            debug!("Finished {}",$n);
            x
        }
    };
);

/// mentini a.k.a. the little mentor most usefull for small functions, methods, nested methods also arrayelements
/// unexplored chaing of calls, parents are liabble/responsible for their childrens
#[macro_export]
macro_rules! mentini {
    ($val:expr) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                debug!("[{}:{}] {} = {:#?}",
                    file!(), line!(), stringify!($val), &tmp);
                tmp
            }
        }
    }
}
