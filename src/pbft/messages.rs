/// Type defining (currently) possible _PBFT messages_ that can be send by
/// replicas or clients.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum PBFTMessage {
    ClientRequest(ClientRequest),
    ClientResponse(ClientResponse),
    PrePrepare(PrePrepareMessage),
    Prepare(PrepareMessage),
    Commit(CommitMessage),
}

/// Type defining a _client request_.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ClientRequest {
    pub operation: u32,
    pub sender_id: u32,
}

/// Type defining a _client response_ message send by replicas after successfully
/// committing locally.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ClientResponse {
    pub result: u32,
    pub sender_id: u32,
}

/// Type defining a _Pre-Prepare_ message send by the _primary_.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct PrePrepareMessage {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32,
}

/// Type defining a _Prepare_ message send by _backups_.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct PrepareMessage {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32,
}

/// Type defining a _Commit_ message send by the _primary_ and _backups_.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct CommitMessage {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32,
}