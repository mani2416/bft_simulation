extern crate log;

use getopts::Options;
use std::env;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use std::fs;

fn main() {
    //    let path_client = "cmd.exe";
    //    let client_args = vec!(
    //        "java",
    //        "-version"
    ////        "-Dlogback.configurationFile=\"./config/logback.xml\"",
    ////        "-cp",
    ////        "\"bin/BFT-SMaRt.jar;lib/*\"",
    //////        "runscripts/smartrun.sh",
    ////        "bftsmart.demo.counter.CounterClient",
    ////        "1001",
    ////        "1",
    ////        "1",
    //    );
    //
    //    //todo start client
    //    let mut client_process = Command::new(path_client)
    //        .args(&client_args)
    //        .current_dir("./client/")
    //        .stdout(Stdio::piped())
    //        .stderr(Stdio::piped())
    //        .output()
    //        .unwrap();
    //
    //    thread::sleep(Duration::from_secs(3));
    //
    ////    let output = client_process.wait_with_output().expect("Failed to read stdout");
    //    println!("status: {}", client_process.status);
    ////    println!("{}", String::from_utf8_lossy(&output.stdout));
    //    io::stdout().write_all(&client_process.stdout).unwrap();
    ////    thread::sleep(Duration::from_secs(10));
    //
    ////    client_process.kill();
    //
    //    return;

    //    let path_bash = "c:/Program Files/Git/usr/bin/mintty.exe";
    let path_bash = "c:/Program Files/Git/git-bash.exe";

    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt("n", "nodes", "number of nodes", "NODES");
    opts.optopt("r", "rounds", "number of rounds", "ROUNDS");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    let nodes = matches.opt_str("n").unwrap().parse::<usize>().unwrap();
    let rounds = matches.opt_str("r").unwrap().parse::<usize>().unwrap();
//    debug!("Starting for n = {}", &nodes);

//    println!("Path to client: {}", path_client);
//    println!("Path to nodes: {:?}", path_nodes);
//    println!("Argument for nodes: {:?}", node_args);

    for r in 1..=rounds {

        let path_client = "./client/";
        let client_args = vec![
            //        "--nodaemon",
            //        "-o",
            //        "AppID=GitForWindows.Bash",
            //        "-o",
            //        "AppLaunchCmd=\"C:\\Program Files\\Git\\git-bash.exe\"",
            //        "-o",
            //        "AppName=\"Git Bash\"",
            //        "-i",
            //        "\"C:\\Program Files\\Git\\git-bash.exe\"",
            //        "--store-taskbar-properties",
            //        "-- /usr/bin/bash",
            //        "--login",
            //        "--i",
            "runscripts/smartrun.sh",
            "bftsmart.demo.counter.CounterClient",
            "1001",
            "1",
            "1",
        ];

        let (sender_kill_client, receiver_kill_client) = mpsc::channel();

        let mut path_nodes = Vec::new();
        let mut node_args = Vec::new();
        let mut vec_sender_kill_node = Vec::new();
        let mut vec_receiver_kill_node = Vec::new();
        for i in 1..=nodes {
            path_nodes.push(format!("./r{}/", i));
            node_args.push(vec![
                "runscripts/smartrun.sh".to_string(),
                "bftsmart.demo.counter.CounterServer".to_string(),
                (i-1).to_string(),
            ]);
            let (tx, rx) = mpsc::channel();
            vec_sender_kill_node.push(tx);
            vec_receiver_kill_node.push(rx);
        }

        // delete the old config gile if it exists
        for path in &path_nodes {
            let mut config = path.to_string();
            config.push_str("config/currentView");
            fs::remove_file(config);
        }

        thread::sleep(Duration::from_millis(500));

        // accept connections and process them serially
        let listener = TcpListener::bind("127.0.0.1:9437").unwrap();
        let (sender_counter, receiver_counter) = mpsc::channel();
        thread::spawn(move || handle_stream(vec_receiver_kill_node, sender_counter, listener, nodes));

        let listener_client = TcpListener::bind("127.0.0.1:9438").unwrap();
        thread::spawn(move || handle_client_client(receiver_kill_client, listener_client));

        //start the services
        let mut vec_process_nodes = Vec::new();

        for _ in 1..=nodes {
            let args = node_args.remove(0);
            let path = path_nodes.remove(0);

            vec_process_nodes.push(
                Command::new(path_bash)
                    .args(args)
                    .current_dir(path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap(),
            );
            thread::sleep(Duration::from_millis(1000));
        }

        let mut cnt = 0;

        loop {
            if let Ok(_) = receiver_counter.recv() {
                cnt += 1;
//                println!("Node serivces ready: {}", cnt)
            }

            if cnt == nodes {
//                println!("All node services indicated 'ready'");
                break;
            }
        }

        let mut client_process = Command::new(path_bash)
            .args(&client_args)
            .current_dir(path_client)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        //    println!("id: {}", client_process.id());

        //wait a bit
        thread::sleep(Duration::from_secs(15));
        //    client_process.kill();

        //send kill signals to all processes
        for sender in vec_sender_kill_node {
            sender.send(()).unwrap();
        }
        sender_kill_client.send(()).unwrap();

        thread::sleep(Duration::from_secs(1));
        if let Ok(_) = client_process.try_wait() {
            println!("Successful round {}", r);
            thread::sleep(Duration::from_secs(10));
        } else {
            println!("Something went wrong shutting down the client");
            break;
        }
    }
}

fn handle_stream(
    mut vec_receiver_kill_node: Vec<Receiver<()>>,
    sender_counter: Sender<()>,
    listener: TcpListener,
    nodes: usize,
) {
    let mut cnt = 0;
    for stream in listener.incoming() {
        cnt += 1;
        let receiver_kill_node = vec_receiver_kill_node.pop().unwrap();
        let sender_counter_clone = sender_counter.clone();
        thread::spawn(move || {
            handle_client(
                receiver_kill_node,
                sender_counter_clone,
                stream.unwrap(),
            )
        });
        if cnt == nodes {
            break;
        }
    }
}

fn handle_client(receiver_kill_node: Receiver<()>, sender_counter: Sender<()>, stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    loop {
        let mut msg = String::new();
        if reader.read_line(&mut msg).is_err() {
//            println!("Reading from socket failed, canceling connection");
            return;
        }
        let msg = msg.trim();

        if msg.eq(&"hello".to_string()) {
//            println!("A node service was activated");
        } else if msg.eq(&"ready".to_string()) {
//            println!("A node service indicated ready");
            sender_counter.send(()).unwrap();
            break;
        }
    }

    if let Ok(_) = receiver_kill_node.recv() {
        writer.write("done".as_bytes()).unwrap();
        writer.flush().unwrap();
    }
//    println!("received kill command for node, listener thread stopping");
}

fn handle_client_client(receiver_kill: Receiver<()>, listener: TcpListener){
    for stream in listener.incoming(){
//        println!("Client connected");
        let stream = stream.unwrap();
        let mut writer = BufWriter::new(stream);

        if let Ok(_) = receiver_kill.recv() {
            writer.write("done".as_bytes()).unwrap();
            writer.flush().unwrap();
        }
        break;
    }
}
