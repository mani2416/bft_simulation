extern crate tini;
extern crate base64;

use tini::Ini;
use std::env;
use log::debug;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::fmt::Debug;


/// get the ini file reference
pub fn get_ini(path: &str) -> Ini{
    Ini::from_file(path).expect("Failed to open ini file")
}

/// exports ini-value to environment
pub fn ini2env(sec: &str, key: &str, conf: &tini::Ini, exp_key: Option<&str>) {
    let msg = format!("no key [{}]{}", sec, key);
    let value: String = conf.get(sec, key).expect(&msg);
    debug!("{}.{} is {}", sec, key, value);
    match exp_key {
        Some(exp_key) => env::set_var(exp_key, value),
        None => env::set_var(sec.to_owned() + "." + key, value),
    }
}

/// expects ini-value to be a ASCII-file and exports its content
pub fn ini2env_filedata(sec: &str, key: &str, conf: &tini::Ini, exp_key: &str) {
    let msg = format!("no key [{}]{}", sec, key);
    let value: String = conf.get(sec, key).expect(&msg);
    debug!("{}.{} is {}", sec, key, value);
    let mut filedata = vec![];
    let msg = format!("{} is not a file", value);
    let mut file = File::open(&value).expect(&msg);
    file.read_to_end(&mut filedata).unwrap();
    let msg = format!("{} is a binary file", value);
    env::set_var(exp_key, &String::from_utf8(filedata).expect(&msg));
}

/// expects ini-value to be a file and exports its content as base64 encoded String
pub fn ini2env_binfiledata(sec: &str, key: &str, conf: &tini::Ini, exp_key: &str) {
    let msg = format!("no key [{}]{}", sec, key);
    let value: String = conf.get(sec, key).expect(&msg);
    debug!("{}.{} is {}", sec, key, value);
    let mut bin_filedata = vec![];
    let msg = format!("{} is not a file", value);
    let mut bin_file = File::open(value).expect(&msg);
    bin_file.read_to_end(&mut bin_filedata).unwrap();
    let filedata_b64 = base64::encode_config(&bin_filedata, base64::STANDARD);
    env::set_var(exp_key, &filedata_b64);
}

/// exports ini-value to environment, given only the path to the ini
pub fn inipath2env(sec: &str, key: &str, file_ini: &str, exp_key: Option<&str>) {
    let ini = Ini::from_file(file_ini).expect("Failed to open ini file");
    let msg = format!("no key [{}]{}", sec, key);
    let value: String = ini.get(sec, key).expect(&msg);
    debug!("{}.{} is {}", sec, key, value);
    match exp_key {
        Some(exp_key) => env::set_var(exp_key, value),
        None => env::set_var(sec.to_owned() + "." + key, value),
    }
}

/// Returns the value from ini
pub fn inipath2var<T>(sec: &str, key: &str, file_ini: &str) -> T
    where T: FromStr,
          <T as FromStr>::Err: Debug
{
    let msg = format!("no key [{}]{}", sec, key);
    Ini::from_file(file_ini).expect("Failed to open ini file").get(sec, key).expect(&msg)
}

/// Returns the value from an environment variable
pub fn env2var<T>(exp_key: &str) -> T
    where T: FromStr,
          <T as FromStr>::Err: Debug
{
    let msg_miss = format!("No {} as environment variable set", exp_key);
    let msg_parse = format!("Can't parse {} into desired variable", exp_key);
    env::var(exp_key).expect(&msg_miss).parse().expect(&msg_parse)
}

/// Returns the vector-value from an environment variable
pub fn env2var_vec<T>(exp_key: &str) -> Vec<T>
    where T: FromStr,
          <T as FromStr>::Err: Debug
{
    let msg_miss = format!("No {} as environment variable set", exp_key);
    let msg_parse = format!("Can't parse {} into desired variable", exp_key);
    let vec_strings = env::var(exp_key).expect(&msg_miss);

    let mut result: Vec<T> = Vec::new();
    for e_string in vec_strings.split_whitespace(){
        result.push(e_string.parse().expect(&msg_parse));
    }
    result
}