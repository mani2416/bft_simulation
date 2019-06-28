use std::collections::{HashMap, HashSet};

use log::warn;

use crate::simulation::config::log_result;
use crate::simulation::time::Time;

use super::messages::*;

/// The output produced by this module. Consumed by the host running the `ReplicaState`.
type Output = Vec<(u32, PBFTMessage)>;

/// Creates an `Output` such that the host broadcasts `msg_out` to all other
/// replicas in the cluster.
fn create_peer_broadcast_output(msg_out: PBFTMessage, peers: &Vec<u32>) -> Output {
    let mut output = Output::with_capacity(peers.len());

    for id in peers {
        output.push((*id, msg_out));
    }

    output
}

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

    pub fn has_commit_quorum_of(&self, quorum_size: usize) -> bool {
        self.commit_quorum.len() >= quorum_size
    }

    pub fn has_prepare_quorum_of(&self, quorum_size: usize) -> bool {
        self.has_pre_prepare_message() && self.prepare_quorum.len() >= quorum_size
    }

    fn has_pre_prepare_message(&self) -> bool {
        if let Some(_) = self.prepare_quorum.iter().find(|msg| match msg {
            PrepareQuorumMessage::PrePrepareMessage(_) => true,
            _ => false,
        }) {
            true
        } else {
            false
        }
    }
}

/// The type defining the state required for participating in a PBFT cluster.
///
/// Exposes a single function for handling incoming PBFT messages.
#[derive(Debug)]
pub struct ReplicaState {
    id: u32,
    log: HashMap<u32, LogEntry>,
    /// For garbage collection purposes we store here IDs of locally
    /// commited requests. This allows us to remove the associated log entry and
    /// ignore all subsequent incoming messages related to the request.
    cl_reqs: HashSet<u32>,
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
    /// The minimal size of a quorum (2 * f + 1) s.t. f < n/3, n = num_of_nodes
    quorum_size: usize,
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

        let nn = num_of_nodes as usize;
        let f: usize = nn / 3 + nn % 3 - 1;
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
            log: HashMap::new(),
            cl_reqs: HashSet::new(),
            last_commited_index: 0,
            peers: (1..=num_of_nodes)
                .into_iter()
                .filter(|i| *i != id)
                .collect(),
            quorum_size: 2 * f + 1 as usize,
        }
    }

    /// Single exposed function that acts as a entry point for handling incoming
    /// messages by peers or clients.
    pub fn handle_message(&mut self, message: PBFTMessage, time: Time) -> Option<Output> {
        // we only process a message if we not already committed locally the
        // associated request
        if self.can_ignore_message(message) {
            return None;
        }

        match message {
            PBFTMessage::ClientRequest(m) => self.handle_client_request(m, time),
            PBFTMessage::PrePrepare(m) => self.handle_pre_prepare_message(m, time),
            PBFTMessage::Prepare(m) => self.handle_prepare_message(m, time),
            PBFTMessage::Commit(m) => self.handle_commit_message(m, time),
            PBFTMessage::ClientResponse(_) => panic!("Replica should not receive a ClientResponse"),
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

    /// Checks if we can ignore the `message`. Returns `true` iff for the associated
    /// request we already committed locally and the incoming message is of type
    /// `PBFTMessage::Prepare` or `PBFTMessage::Commit`
    fn can_ignore_message(&self, message: PBFTMessage) -> bool {
        match message {
            PBFTMessage::Prepare(m) => self.cl_reqs.contains(&m.c_req.operation),
            PBFTMessage::Commit(m) => self.cl_reqs.contains(&m.c_req.operation),
            _ => false,
        }
    }

    /// Updates the predicates for a log entry associated with the `req_id`.
    fn update_prediactes(&mut self, req_id: u32, mut output: Output, time: Time) -> Option<Output> {
        let entry = self.log.get_mut(&req_id).unwrap();

        // `prepared` predicate check
        if !entry.prepared && entry.has_prepare_quorum_of(self.quorum_size) {
            log_result(
                time,
                Some(self.id),
                &format!("{};prepared", entry.client_request.operation),
            );

            entry.prepared = true;

            let commit =
                CommitMessage::new(entry.client_request, entry.view, entry.seq_number, self.id);

            entry.commit_quorum.insert(commit);

            // send batch of commit messages since we prepared
            output.append(&mut create_peer_broadcast_output(
                PBFTMessage::Commit(commit),
                &self.peers,
            ));
        }

        // `committed_local` prediacte check
        if entry.prepared && !entry.committed_local && entry.has_commit_quorum_of(self.quorum_size)
        {
            log_result(
                time,
                Some(self.id),
                &format!("{};committed_local", entry.client_request.operation),
            );

            entry.committed_local = true;

            // we don't need the entry anymore. Therefore, remove it from the log
            self.log.remove(&req_id);
            // update the committed local set so we ignore subsequent incoming messages
            // related to this request
            self.cl_reqs.insert(req_id);
        }

        match output.len() {
            0 => None,
            _ => Some(output),
        }
    }

    /// Handles incoming client requests.
    fn handle_client_request(&mut self, msg_in: ClientRequest, time: Time) -> Option<Output> {
        if self.is_primary() {
            log_result(
                time,
                Some(self.id),
                &format!("{};request", msg_in.operation),
            );

            let seq_number = self.next_seq_num();
            let mut entry = LogEntry::new(self.current_view, seq_number, msg_in);
            let preprepare = PrePrepareMessage::new(msg_in, self.current_view, seq_number, self.id);

            entry
                .prepare_quorum
                .insert(PrepareQuorumMessage::PrePrepareMessage(preprepare));

            self.log.insert(msg_in.operation, entry);

            return Some(create_peer_broadcast_output(
                PBFTMessage::PrePrepare(preprepare),
                &self.peers,
            ));
        }

        warn!(target: "node", "Non-primary PBFTNode {} received a client request", self.id);

        None
    }

    fn handle_pre_prepare_message(
        &mut self,
        msg_in: PrePrepareMessage,
        time: Time,
    ) -> Option<Output> {
        if self.curr_primary() == msg_in.sender_id {
            let req_id = msg_in.c_req.operation;
            let entry = match self.log.get_mut(&req_id) {
                Some(entry) => entry,
                None => {
                    self.log.insert(
                        req_id,
                        LogEntry::new(msg_in.view, msg_in.seq_number, msg_in.c_req),
                    );
                    self.log.get_mut(&req_id).unwrap()
                }
            };

            log_result(time, Some(self.id), &format!("{};pre-prepared", req_id));

            let prepare =
                PrepareMessage::new(entry.client_request, entry.view, entry.seq_number, self.id);

            entry
                .prepare_quorum
                .insert(PrepareQuorumMessage::PrePrepareMessage(msg_in));
            entry
                .prepare_quorum
                .insert(PrepareQuorumMessage::PrepareMessage(prepare));

            let output = create_peer_broadcast_output(PBFTMessage::Prepare(prepare), &self.peers);

            return self.update_prediactes(req_id, output, time);
        }

        warn!(target:"node", "PBFTNode {} received a PrePrepare message from non-primary peer {}", self.id, msg_in.sender_id);

        None
    }

    fn handle_prepare_message(&mut self, msg_in: PrepareMessage, time: Time) -> Option<Output> {
        let req_id = msg_in.c_req.operation;

        match self.log.get_mut(&req_id) {
            Some(entry) => {
                entry
                    .prepare_quorum
                    .insert(PrepareQuorumMessage::PrepareMessage(msg_in));

                return self.update_prediactes(req_id, Output::new(), time);
            }
            None => {
                let mut entry = LogEntry::new(msg_in.view, msg_in.seq_number, msg_in.c_req);

                entry
                    .prepare_quorum
                    .insert(PrepareQuorumMessage::PrepareMessage(msg_in));

                self.log.insert(msg_in.c_req.operation, entry);
            }
        };
        None
    }

    fn handle_commit_message(&mut self, msg_in: CommitMessage, time: Time) -> Option<Output> {
        let req_id = msg_in.c_req.operation;

        match self.log.get_mut(&req_id) {
            Some(entry) => {
                entry.commit_quorum.insert(msg_in);

                return self.update_prediactes(req_id, Output::new(), time);
            }
            None => {
                let mut entry = LogEntry::new(msg_in.view, msg_in.seq_number, msg_in.c_req);

                entry.commit_quorum.insert(msg_in);
                self.log.insert(msg_in.c_req.operation, entry);
            }
        }
        None
    }
}

/*******************************************************************************
 * TESTS
 ******************************************************************************/

#[cfg(test)]
mod tests {
    use crate::simulation::time::Time;

    use super::*;

    #[test]
    fn require_prepreare_message_in_prepare_quorum() {
        let num_of_nodes = 4;
        let f = 1;
        let quorum_size = 2 * f + 1;

        let mut state = ReplicaState::new(1337, num_of_nodes);

        let c_req = ClientRequest {
            operation: 0,
            sender_id: 0,
        };
        let mut prepare_msg = PrepareMessage {
            c_req,
            view: 1,
            seq_number: 1,
            sender_id: 1,
        };

        for i in 1..=3 {
            prepare_msg.sender_id = i;
            state.handle_prepare_message(prepare_msg, Time::new(32));
        }

        if let Some(entry) = state.log.get(&c_req.operation) {
            assert!(entry.prepare_quorum.len() >= quorum_size as usize);
            assert_eq!(entry.has_prepare_quorum_of(quorum_size), false);
            assert_eq!(entry.prepared, false);
        } else {
            panic!("Entry should exist!");
        }
    }

    #[test]
    fn state_transition_from_prepared_to_committed() {
        let num_of_nodes = 4;
        let mut state = ReplicaState::new(1337, num_of_nodes);
        let c_req = ClientRequest {
            operation: 0,
            sender_id: 0,
        };
        let mut commit_msg = CommitMessage {
            c_req,
            view: 1,
            seq_number: 1,
            sender_id: 1,
        };

        for i in 1..num_of_nodes {
            commit_msg.sender_id = i;
            state.handle_commit_message(commit_msg, Time::new(32));
        }

        // we cannot commit locally without being prepared, although we might have
        // a commit quorum present
        if let Some(entry) = state.log.get(&c_req.operation) {
            assert_eq!(entry.committed_local, false);
            assert_eq!(entry.prepared, false);
            assert_eq!(entry.has_commit_quorum_of(state.quorum_size), true);
        } else {
            panic!("Entry should exist!");
        }

        state.handle_pre_prepare_message(
            PrePrepareMessage {
                c_req,
                view: 1,
                seq_number: 1,
                sender_id: 1,
            },
            Time::new(32),
        );

        let mut prepare_msg = PrepareMessage {
            c_req,
            view: 1,
            seq_number: 1,
            sender_id: 1,
        };

        for i in 1..num_of_nodes {
            prepare_msg.sender_id = i;
            state.handle_prepare_message(prepare_msg, Time::new(32));
        }

        // after becoming prepared and having a commit quorum collected we
        // can finally commit locally
        if let Some(entry) = state.log.get(&c_req.operation) {
            assert_eq!(entry.prepared, true);
            assert_eq!(entry.committed_local, true);
        } else {
            panic!("Entry should exist!");
        }
    }
}
