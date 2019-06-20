use std::collections::HashSet;

use log::{debug, warn};

use crate::simulation::config::log_result;

use super::messages::*;

/// The output produced by this module. Consumed by the host running the `ReplicaState`.
type Output = Option<Vec<(u32, PBFTMessage)>>;

/// The type defining allowed Prepare (1st) quorum messages.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
enum PrepareQuorumMessage {
    PrePrepareMessage(PrePrepareMessage),
    PrepareMessage(PrepareMessage),
}

/// The type defining allowed roles for replicas.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplicaRole {
    Primary,
    Backup,
}

// TODO: The current structure of storing messages works good for the "happy-path",
//       but does not tolerate if messages arrive out of order / get lost...
//       Therefore, improve the way of storing messages!!

/// The type defining an entry of the replica's log. An entry stores the request
/// and all related information required by the protocol.
///
/// Thus, holds the `view` and `seq_number` assigned by the primary for the client request
/// as well as all associated messages like the original _client request_ and
/// quorum messages for the _Prepare_ and the _Commit_ quorum.
#[derive(Debug)]
pub struct LogEntry {
    /// View number assigned by the primary to this request
    view: u64,
    /// Sequence number assigned by the to this request
    seq_number: u64,
    // -------------------- Associated Messages --------------------
    /// The original client request
    client_request: ClientRequest,
    /// All prepare quorum messages for this entry
    prepare_quorum: HashSet<PrepareQuorumMessage>,
    /// All commit quorum messages for this entry
    commit_quorum: HashSet<CommitMessage>,
    // --------------------     Predicates      --------------------
    /// `true` as soon as replica has collected a _Prepare_ quorum for this entry.
    prepared: bool,
    /// `true` as soon as replica has collected a _Commit_ quorum for this entry.
    committed_local: bool,
}

impl LogEntry {
    pub fn new(view: u64, seq_number: u64, client_request: ClientRequest) -> Self {
        LogEntry {
            view,
            seq_number,
            client_request,
            prepare_quorum: HashSet::new(),
            commit_quorum: HashSet::new(),
            committed_local: false,
            prepared: false,
        }
    }
}

/// The type defining the state required for participating in a PBFT cluster.
///
/// Exposes a single function for handling incoming PBFT messages.
#[derive(Debug)]
pub struct ReplicaState {
    id: u32,
    log: Vec<LogEntry>,
    /// The fixed number of nodes participating in the cluster.
    num_of_nodes: u32,
    /// The view number in which the replica currently operates.
    current_view: u64,
    /// Used only by the `primary` to assign the next sequence number.
    next_seq_num: u64,
    /// The index of the last committed entry in the `log`.
    last_commited_index: usize,
    /// Specifies the current role of the replica in the cluster.
    role: ReplicaRole,
    /// Holds the IDs of other peers.
    peers: Vec<u32>,
    /// Specifies the resilience given the `num_of_nodes` (= 3f + 1).
    f: u32,
}

impl ReplicaState {
    /// Creates a new `ReplicaState` with `current_view` set to 1. Thus,
    /// the (fixed) primary is always the node with id `1`.
    ///
    /// Requires the parameter `num_of_nodes` to be at least `4`, otherwise it
    /// `panics!` since at least 4 nodes are required for successful operation.
    pub fn new(id: u32, num_of_nodes: u32) -> Self {
        if num_of_nodes < 4 {
            panic!("Need at least 4 PBFT nodes but got only {}", num_of_nodes);
        }

        let initial_view = 1;

        ReplicaState {
            id,
            num_of_nodes,
            role: match id == (initial_view % num_of_nodes) {
                true => ReplicaRole::Primary,
                false => ReplicaRole::Backup,
            },
            current_view: initial_view as u64,
            next_seq_num: 0,
            log: Vec::new(),
            last_commited_index: 0,
            peers: (1..num_of_nodes + 1)
                .into_iter()
                .filter(|i| *i != id)
                .collect(),
            f: num_of_nodes / 3 + num_of_nodes % 3 - 1,
        }
    }

    /// Single exposed function that acts as a entry point for handling incoming
    /// messages by peers or clients.
    pub fn handle_message(&mut self, message: PBFTMessage) -> Output {
        match message {
            PBFTMessage::ClientRequest(m) => self.handle_client_request(m),
            PBFTMessage::PrePrepare(m) => self.handle_pre_prepare_message(m),
            PBFTMessage::Prepare(m) => self.handle_prepare_message(m),
            PBFTMessage::Commit(m) => self.handle_commit_message(m),
            _ => None,
        }
    }

    /// Gets the `id` of the primary for the current view.
    fn curr_primary(&self) -> u32 {
        (self.current_view % (self.num_of_nodes as u64)) as u32
    }

    /// Checks if `self` is the primary for the current view.
    fn is_primary(&self) -> bool {
        self.role == ReplicaRole::Primary
    }

    /// Increments the sequence number counter and returns the value.
    fn next_seq_num(&mut self) -> u64 {
        self.next_seq_num += 1;
        self.next_seq_num
    }

    /// Creates an `Output` such that the host broadcasts `msg_out` to all other
    /// replicas in the cluster.
    fn create_peer_broadcast_output(&self, msg_out: PBFTMessage) -> Output {
        let mut output = Vec::<(u32, PBFTMessage)>::with_capacity(self.peers.len());

        for id in &self.peers {
            output.push((*id, msg_out));
        }

        return Some(output);
    }

    /// Handles incoming client requests.
    fn handle_client_request(&mut self, msg_in: ClientRequest) -> Output {
        if self.is_primary() {
            // TODO: needs more validations before processing

            let seq_number = self.next_seq_num();
            let mut entry = LogEntry::new(self.current_view, seq_number, msg_in);
            let preprepare = PrePrepareMessage {
                view: self.current_view,
                seq_number,
                sender_id: self.id,
                c_req: msg_in,
            };

            entry
                .prepare_quorum
                .insert(PrepareQuorumMessage::PrePrepareMessage(preprepare));

            self.log.push(entry);

            return self.create_peer_broadcast_output(PBFTMessage::PrePrepare(preprepare));
        } else {
            warn!(target: "node", "Non-primary PBFTNode {} received a client request", self.id);
        }
        None
    }

    fn handle_pre_prepare_message(&mut self, msg_in: PrePrepareMessage) -> Output {
        // TODO: needs validations before processing
        if self.curr_primary() == msg_in.sender_id {
            let mut entry = LogEntry::new(msg_in.view, msg_in.seq_number, msg_in.c_req);

            let prepare = PrepareMessage {
                view: msg_in.view,
                seq_number: msg_in.seq_number,
                sender_id: self.id,
                c_req: msg_in.c_req,
            };

            entry
                .prepare_quorum
                .insert(PrepareQuorumMessage::PrePrepareMessage(msg_in));
            entry
                .prepare_quorum
                .insert(PrepareQuorumMessage::PrepareMessage(prepare));

            self.log.push(entry);

            return self.create_peer_broadcast_output(PBFTMessage::Prepare(prepare));
        } else {
            warn!(target:"node", "PBFTNode {} received a PrePrepare message from non-primary peer {}", self.id, msg_in.sender_id);
        }
        None
    }

    fn handle_prepare_message(&mut self, message: PrepareMessage) -> Output {
        // TODO: we assume seq_number - 1 == index. Eventually, won't work anymore. Therefore, FIX!
        // correct, when we add delays or message omissions the messages will arrive out of order and uninitialized indices might be called
        let entry = &mut self.log[(message.seq_number - 1) as usize];

        // TODO: needs validations before processing

        entry
            .prepare_quorum
            .insert(PrepareQuorumMessage::PrepareMessage(message));

        if !entry.prepared && entry.prepare_quorum.len() >= (2 * self.f + 1) as usize {
            debug!(target:"node", "PBFTNode {} successfully prepared for seq_number {}", self.id, message.seq_number);
            // TODO make an entry for the result logger. The state still needs to be given the time of the reception event, so you can log it here.
            // Call should be something like this: debug!(create_log_result_message(<time>, Some(self.id), "prepared")); // or "prepare quorum completed"
            entry.prepared = true;

            let commit = CommitMessage {
                view: entry.view,
                seq_number: entry.seq_number,
                c_req: entry.client_request,
                sender_id: self.id,
            };

            entry.commit_quorum.insert(commit);

            return self.create_peer_broadcast_output(PBFTMessage::Commit(commit));
        }
        None
    }

    fn handle_commit_message(&mut self, message: CommitMessage) -> Output {
        // TODO: we assume seq_number - 1 == index. Eventually, won't work anymore. Therefore, FIX!
        let entry = &mut self.log[(message.seq_number - 1) as usize];

        // TODO: needs validations before processing

        entry.commit_quorum.insert(message);

        if !entry.committed_local && entry.commit_quorum.len() >= (2 * self.f + 1) as usize {
            debug!(target:"node", "PBFTNode {} successfully committed locally for seq_number {} ", self.id, message.seq_number);
            entry.committed_local = true;

            let response = ClientResponse {
                result: entry.client_request.operation,
                sender_id: self.id,
            };

            println!(
                "PBFTNode {} would send to client response {:?}",
                self.id, response
            );
        }
        None
    }
}
