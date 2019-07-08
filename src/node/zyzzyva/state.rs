use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use super::messages::*;
use crate::simulation::config::log_result;
use crate::simulation::time::Time;

pub const CLIENT_ID: u32 = 2;

type Output = Vec<(u32, ZyzzyvaMessage)>;

/// Creates an `Output` such that the host broadcasts `msg_out` to all other
/// replicas in the cluster.
fn create_peer_broadcast_output(msg_out: ZyzzyvaMessage, peers: &Vec<u32>) -> Output {
    let mut output = Output::with_capacity(peers.len());

    for id in peers {
        output.push((*id, msg_out.clone()));
    }

    output
}

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
    commit_certificate: HashSet<SpeculativeResponse>,
    local_commits: HashSet<u32>,
    speculative_execution: bool,
    committed_local: bool,
    completed: bool,
    timed_out: bool,
}

impl LogEntry {
    pub fn new(c_req: ClientRequest, view: u64, seq_number: u64) -> Self {
        LogEntry {
            c_req,
            view,
            seq_number,
            commit_certificate: HashSet::new(),
            local_commits: HashSet::new(),
            speculative_execution: false,
            committed_local: false,
            completed: false,
            timed_out: false,
        }
    }
}

#[derive(Debug)]
pub struct State {
    id: u32,
    log: HashMap<u32, LogEntry>,
    /// For garbage collection purposes we store here IDs of locally
    /// commited requests. This allows us to remove the associated log entry and
    /// ignore all subsequent incoming messages related to the request.
    cl_reqs: HashSet<u32>,
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
            cl_reqs: HashSet::new(),
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

    pub fn handle_message(
        &mut self,
        zyzzyva_message: ZyzzyvaMessage,
        time: Time,
    ) -> Option<Output> {
        // we only process a message if we not already committed locally the
        // associated request
        if self.can_ignore_message(&zyzzyva_message) {
            return None;
        }

        match zyzzyva_message {
            ZyzzyvaMessage::ClientRequest(m) => self.handle_client_request(m, time),
            ZyzzyvaMessage::ClientTimeout(m) => self.handle_client_timeout(m, time),
            ZyzzyvaMessage::OrderRequest(m) => self.handle_order_request(m, time),
            ZyzzyvaMessage::SpeculativeResponse(m) => self.handle_speculative_response(m, time),
            ZyzzyvaMessage::Commit(m) => self.handle_commit(m, time),
            ZyzzyvaMessage::LocalCommit(m) => self.handle_local_commit(m, time),
        }
    }

    fn can_ignore_message(&self, message: &ZyzzyvaMessage) -> bool {
        match message {
            ZyzzyvaMessage::LocalCommit(m) => self.cl_reqs.contains(&m.c_req.operation),
            ZyzzyvaMessage::SpeculativeResponse(m) => self.cl_reqs.contains(&m.c_req.operation),
            ZyzzyvaMessage::OrderRequest(m) => self.cl_reqs.contains(&m.c_req.operation),
            _ => false,
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

    fn gc_entry(&mut self, req_id: u32) {
        // we don't need the entry anymore. Therefore, remove it from the log
        self.log.remove(&req_id);
        // update the committed local set so we ignore subsequent incoming messages
        // related to this request
        self.cl_reqs.insert(req_id);
    }

    fn handle_local_commit(&mut self, msg_in: LocalCommit, time: Time) -> Option<Output> {
        match self.log.get_mut(&msg_in.c_req.operation) {
            Some(entry) => {
                entry.local_commits.insert(msg_in.sender_id);

                if entry.local_commits.len() >= self.quorum_size && !entry.completed {
                    log_result(
                        time,
                        Some(self.id),
                        &format!("{};completed", msg_in.c_req.operation),
                    );
                    // entry.completed = true;
                    let id = entry.c_req.operation;
                    self.gc_entry(id);
                }
            }
            None => panic!(
                "Received a local commit message for entry {} that is not stored at the client",
                msg_in.c_req.operation
            ),
        }
        None
    }

    fn handle_client_timeout(&mut self, msg_in: ClientTimeout, time: Time) -> Option<Output> {
        if self.role == Role::Client {
            if let Some(entry) = self.log.get_mut(&msg_in.req_id) {
                entry.timed_out = true;
                let cert_len = entry.commit_certificate.len();

                // Zyzzyva 4.b
                if cert_len >= self.quorum_size && cert_len < self.peers.len() {
                    return Some(create_peer_broadcast_output(
                        ZyzzyvaMessage::Commit(Commit::new(
                            msg_in.req_id,
                            entry.commit_certificate.clone().into_iter().collect(),
                            self.id,
                        )),
                        &self.peers,
                    ));
                }

                // Zyzzyva 4.c
                if cert_len < self.quorum_size {
                    log_result(time, Some(self.id), &format!("{};timed-out", msg_in.req_id));
                }
            }
        } else {
            panic!(
                "Non-client node {} received a ClientTimeout message",
                self.id
            );
        }

        None
    }

    fn handle_client_request(&mut self, msg_in: ClientRequest, time: Time) -> Option<Output> {
        match self.role {
            // The client will receive the request from the simulation and create
            // a "real" request to the primary
            Role::Client => {
                let request = ClientRequest::new(msg_in.operation, self.id);
                let entry = LogEntry::new(request, 0, 0);
                let mut output = Output::with_capacity(2);

                self.log.insert(msg_in.operation, entry);

                output.push((self.curr_primary(), ZyzzyvaMessage::ClientRequest(request)));
                // add a timeout event for the client itself.
                output.push((
                    self.id,
                    ZyzzyvaMessage::ClientTimeout(ClientTimeout::new(msg_in.operation)),
                ));

                return Some(output);
            }
            Role::Primary => {
                let seq_number = self.next_seq_num();
                let mut entry = LogEntry::new(msg_in, self.current_view, seq_number);
                let mut output = Output::with_capacity(self.peers.len() + 1);

                log_result(
                    time,
                    Some(self.id),
                    &format!("{};speculative_commit", msg_in.operation),
                );

                entry.speculative_execution = true;
                self.log.insert(msg_in.operation, entry);

                output.push((
                    CLIENT_ID,
                    ZyzzyvaMessage::SpeculativeResponse(SpeculativeResponse::new(
                        msg_in,
                        self.current_view,
                        seq_number,
                        self.id,
                    )),
                ));

                output.append(&mut create_peer_broadcast_output(
                    ZyzzyvaMessage::OrderRequest(OrderRequest::new(
                        msg_in,
                        self.current_view,
                        seq_number,
                        self.id,
                    )),
                    &self.peers,
                ));

                return Some(output);
            }
            Role::Backup => panic!("Backup received client request {:?}", msg_in),
        }
    }

    fn handle_order_request(&mut self, msg_in: OrderRequest, time: Time) -> Option<Output> {
        match self.role {
            Role::Backup => match self.log.get(&msg_in.c_req.operation) {
                Some(_) => panic!(
                    "Received a OrderRequest for operation {} although there is already an entry. {:?}",
                    msg_in.c_req.operation,
                    msg_in
                ),
                None => {
                    let mut entry = LogEntry::new(msg_in.c_req, msg_in.view, msg_in.seq_number);

                    entry.speculative_execution = true;

                    self.log.insert(msg_in.c_req.operation, entry);

                    log_result(
                        time,
                        Some(self.id),
                        &format!("{};speculative_commit", msg_in.c_req.operation),
                    );

                    return Some(vec![(
                        CLIENT_ID,
                        ZyzzyvaMessage::SpeculativeResponse(SpeculativeResponse::new(
                            msg_in.c_req,
                            msg_in.view,
                            msg_in.seq_number,
                            self.id,
                        )),
                    )]);
                }
            },
            _ => {
                panic!(
                    "Only Backups may received a Order Request! But node {} received one!",
                    self.id
                );
            }
        }
    }

    fn handle_speculative_response(
        &mut self,
        msg_in: SpeculativeResponse,
        time: Time,
    ) -> Option<Output> {
        match self.role {
            Role::Client => {
                match self.log.get_mut(&msg_in.c_req.operation) {
                    Some(entry) => {
                        // in case we timed-out we only accept commit messages
                        // for the associated request
                        if entry.timed_out {
                            return None;
                        }

                        let cert = &mut entry.commit_certificate;
                        cert.insert(msg_in);

                        if cert.len() == self.quorum_size {
                            log_result(
                                time,
                                Some(self.id),
                                &format!("{};commit_certificate", msg_in.c_req.operation),
                            );
                        }

                        // Zyzzyva 4.a
                        if cert.len() == self.num_of_nodes as usize {
                            log_result(
                                time,
                                Some(self.id),
                                &format!("{};completed", msg_in.c_req.operation),
                            );
                            // entry.completed = true;

                            let req_id = entry.c_req.operation;
                            self.gc_entry(req_id);
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

    fn handle_commit(&mut self, msg_in: Commit, time: Time) -> Option<Output> {
        match self.role {
            Role::Client => panic!("A client should not receive a Commit message!"),
            _ => {
                if let Some(entry) = self.log.get_mut(&msg_in.req_id) {
                    entry.commit_certificate = HashSet::from_iter(msg_in.certificate.into_iter());
                    entry.committed_local = true;

                    let mut output = Output::new();

                    output.push((
                        CLIENT_ID,
                        ZyzzyvaMessage::LocalCommit(LocalCommit::new(
                            entry.c_req,
                            entry.view,
                            entry.seq_number,
                            self.id,
                        )),
                    ));
                    self.gc_entry(msg_in.req_id);
                    return Some(output);
                } else {
                    let spec_res = msg_in.certificate[0];
                    let mut entry =
                        LogEntry::new(spec_res.c_req, spec_res.view, spec_res.seq_number);
                    entry.commit_certificate = HashSet::from_iter(msg_in.certificate.into_iter());
                    entry.committed_local = true;

                    log_result(
                        time,
                        Some(self.id),
                        &format!("{};committed_local", entry.c_req.operation),
                    );

                    let mut output = Output::with_capacity(1);

                    output.push((
                        CLIENT_ID,
                        ZyzzyvaMessage::LocalCommit(LocalCommit::new(
                            entry.c_req,
                            entry.view,
                            entry.seq_number,
                            self.id,
                        )),
                    ));

                    // self.log.insert(msg_in.req_id, entry);
                    let req_id = msg_in.req_id;
                    self.gc_entry(req_id);

                    return Some(output);
                }
                // let output = self.process_history(time);

                // return match output.len() {
                //     0 => None,
                //     _ => Some(output),
                // };
            }
        }
    }

    /// This method is used when requiring a consistent history before executing
    /// a order request. Zyzzyva relies on such behaviour, however, to allow for
    /// a fair comparison between our PBFT we should not require such behaviour,
    /// but instead, send out a speculative response or local commit message as
    /// soon as we receive the corresponding request.
    fn _process_history(&mut self, time: Time) -> Output {
        let mut output = Output::new();

        loop {
            let lc_sn = self.lc_seq_num;
            if let Some(entry) = self
                .log
                .values_mut()
                .find(|entry| entry.seq_number == lc_sn + 1)
            {
                if entry.commit_certificate.len() >= self.quorum_size {
                    entry.committed_local = true;
                    log_result(
                        time,
                        Some(self.id),
                        &format!("{};committed_local", entry.c_req.operation),
                    );

                    output.push((
                        CLIENT_ID,
                        ZyzzyvaMessage::LocalCommit(LocalCommit::new(
                            entry.c_req,
                            entry.view,
                            entry.seq_number,
                            self.id,
                        )),
                    ));
                } else {
                    entry.speculative_execution = true;
                    log_result(
                        time,
                        Some(self.id),
                        &format!("{};speculative_commit", entry.c_req.operation),
                    );
                    output.push((
                        CLIENT_ID,
                        ZyzzyvaMessage::SpeculativeResponse(SpeculativeResponse::new(
                            entry.c_req,
                            entry.view,
                            entry.seq_number,
                            self.id,
                        )),
                    ));
                }

                self.lc_seq_num += 1;
                continue;
            }
            break;
        }
        output
    }
}
