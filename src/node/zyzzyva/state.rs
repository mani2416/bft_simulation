use std::collections::{HashMap, HashSet};

use super::messages::*;

use crate::simulation::config::log_result;
use crate::simulation::time::Time;

/**
 * FIX:
 * ----------
 * 1. Client requests go through our simulation network, this is not the case
 * in PBFT. Requests may get lost.
 * 2. Properly model the client
 * 3. How to properly send client requests?
 * 4. How to realise a client timer for eventually sending a commit certificate?
 *      - put nodes in separate threads?
 *
 *
 */


pub const CLIENT_ID: u32 = 2;

type Output = Option<Vec<(u32, ZyzzyvaMessage)>>;

#[derive(Debug, PartialEq, Eq)]
pub enum Role {
    Client,
    Primary,
    Backup,
}

#[derive(Debug)]
pub struct LogEntry {
    c_req: ClientRequest,
    view: u64,
    seq_number: u64,
    commit_certificate: Option<HashSet<SpeculativeResponse>>,
    speculative_execution: bool,
}

impl LogEntry {
    pub fn new(c_req: ClientRequest, view: u64, seq_number: u64) -> Self {
        LogEntry {
            c_req,
            view,
            seq_number,
            commit_certificate: None,
            speculative_execution: false,
        }
    }
}


#[derive(Debug)]
pub struct State {
    id: u32,
    log: HashMap<u32, LogEntry>,
    num_of_nodes: u32,
    current_view: u64,
    next_seq_num: u64,
    role: Role,
    peers: Vec<u32>,
    client_id: u32,
    quorum_size: usize,
    lc_seq_num: u64,
}

impl State {
    pub fn new(id: u32, num_of_nodes: u32) -> Self {
        if num_of_nodes < 5 {
            panic!(
                "Need 5 Zyzzyva nodes (client is part of the cluster) but got only {}",
                num_of_nodes
            );
        }

        // NOTE: as for now we model a client as a node and thus require
        // for the simulation to specify 5 nodes so it also creates a client
        // node. Interanlly we work with 4 replicas, therefore we subtract 1.
        let num_of_nodes = num_of_nodes - 1;

        let nn = num_of_nodes as usize;
        let f: usize = nn / 3 + nn % 3 - 1;

        State {
            id,
            log: HashMap::new(),
            num_of_nodes,
            current_view: 1,
            next_seq_num: 0,
            lc_seq_num: 0,
            role: match id {
                1 => Role::Primary,
                CLIENT_ID => Role::Client,
                _ => Role::Backup,
            },
            client_id: CLIENT_ID,
            peers: (1..=num_of_nodes + 1)
                .into_iter()
                .filter(|i| *i != id && *i != CLIENT_ID)
                .collect(),
            quorum_size: 2 * f + 1,
        }
    }

    pub fn handle_message(&mut self, zyzzyva_message: ZyzzyvaMessage, time: Time) -> Output {
        match zyzzyva_message {
            ZyzzyvaMessage::ClientRequest(m) => self.handle_client_request(m, time),
            ZyzzyvaMessage::OrderRequest(m) => self.handle_order_request(m, time),
            ZyzzyvaMessage::SpeculativeResponse(m) => self.handle_speculative_response(m, time),
            ZyzzyvaMessage::Commit(m) => self.handle_commit(m, time),
        }
    }

    /// Gets the `id` of the primary for the current view.
    fn curr_primary(&self) -> u32 {
        (self.current_view % (self.num_of_nodes as u64)) as u32
    }

    /// Increments the sequence number counter and returns the value.
    fn next_seq_num(&mut self) -> u64 {
        self.next_seq_num += 1;
        self.next_seq_num
    }

    fn handle_client_request(&mut self, msg_in: ClientRequest, time: Time) -> Output {
        match self.role {
            // The client will receive the request from the simulation and create
            // a "real" request to the primary
            Role::Client => {
                let request = ClientRequest {
                    operation: msg_in.operation,
                    sender_id: self.id,
                };
                let mut entry = LogEntry::new(request, 0, 0);
                // only client initializes this
                entry.commit_certificate = Some(HashSet::new());

                self.log.insert(msg_in.operation, entry);

                return Some(vec![(
                    self.curr_primary(),
                    ZyzzyvaMessage::ClientRequest(request),
                )]);
            }
            Role::Primary => {

                let seq_number = self.next_seq_num();

                let mut entry = LogEntry::new(msg_in, self.current_view, seq_number);
                let mut output = Vec::<(u32, ZyzzyvaMessage)>::with_capacity(self.peers.len() + 1);

                // if our history is complete, we can speculatively execute the request
                if self.lc_seq_num + 1 == seq_number {
                    log_result(
                        time,
                        Some(self.id),
                        &format!("{};speculative_commit", msg_in.operation),
                    );

                    self.lc_seq_num += 1;
                    entry.speculative_execution = true;

                    output.push((
                        CLIENT_ID,
                        ZyzzyvaMessage::SpeculativeResponse(SpeculativeResponse {
                            c_req: msg_in,
                            view: self.current_view,
                            seq_number,
                            sender_id: self.id,
                        }),
                    ));
                }

                self.log.insert(msg_in.operation, entry);


                // broadcast order request
                for i in &self.peers {
                    output.push((
                        *i,
                        ZyzzyvaMessage::OrderRequest(OrderRequest {
                            c_req: msg_in,
                            view: self.current_view,
                            seq_number,
                            sender_id: self.id,
                        }),
                    ))
                }

                return Some(output);
            }
            Role::Backup => panic!("Backup received client request {:?}", msg_in),
        }
    }

    fn handle_order_request(&mut self, msg_in: OrderRequest, time: Time) -> Output {
        match self.role {
            Role::Backup => {
                match self.log.get(&msg_in.c_req.operation) {
                    Some(_) => panic!(
                        "Received a subsequent OrderRequest for the same operation. {:?}",
                        msg_in
                    ),
                    None => {
                        // TODO: add some validations
                        self.log.insert(
                            msg_in.c_req.operation,
                            LogEntry::new(msg_in.c_req, msg_in.view, msg_in.seq_number),
                        );

                        let mut result = Vec::<(u32, ZyzzyvaMessage)>::new();

                        // we execute all requests for which we potentially filled gaps by receiving the current OrderRequest
                        loop {
                            let lc_sn = self.lc_seq_num;
                            if let Some(entry) = self
                                .log
                                .values_mut()
                                .find(|entry| entry.seq_number == lc_sn + 1)
                            {
                                self.lc_seq_num += 1;
                                entry.speculative_execution = true;
                                log_result(
                                    time,
                                    Some(self.id),
                                    &format!("{};speculative_commit", entry.c_req.operation),
                                );

                                result.push((
                                    CLIENT_ID,
                                    ZyzzyvaMessage::SpeculativeResponse(SpeculativeResponse {
                                        c_req: entry.c_req,
                                        view: entry.view,
                                        seq_number: entry.seq_number,
                                        sender_id: self.id,
                                    }),
                                ));
                                continue;
                            }
                            break;
                        }

                        return match result.len() {
                            0 => None,
                            _ => Some(result),
                        };
                    }
                }
            }
            _ => {
                panic!(
                    "Only Backups may received a Order Request! But node {} received one!",
                    self.id
                );
            }
        }
    }

    fn handle_speculative_response(&mut self, msg_in: SpeculativeResponse, time: Time) -> Output {
        match self.role {
            Role::Client => {
                match self.log.get_mut(&msg_in.c_req.operation) {
                    Some(entry) => {
                        if let Some(cert) = &mut entry.commit_certificate {
                            cert.insert(msg_in);

                            if cert.len() == self.num_of_nodes as usize {
                                log_result(
                                    time,
                                    Some(self.id),
                                    &format!("{};commit_certificate", msg_in.c_req.operation),
                                );
                            }
                        } else {
                            panic!("Client's commit certificate was not initialized");
                        }
                    }
                    None => {
                        panic!("Received a speculative response for an operation that was not requested!");
                    }
                }

            }
            _ => panic!(
                "Only the Client should receive a Speculative Reponse! But node {} received one!",
                self.id
            ),
        }
        None
    }

    fn handle_commit(&mut self, _msg_in: Commit, _time: Time) -> Output {
        match self.role {
            Role::Client => panic!("A client should not receive a Commit message!"),
            _ => {
                println!("{} received a Commit message", self.id);
            }
        }
        None
    }
}