#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ZyzzyvaMessage {
    ClientRequest(ClientRequest),
    ClientTimeout(ClientTimeout),
    OrderRequest(OrderRequest),
    SpeculativeResponse(SpeculativeResponse),
    Commit(Commit),
    LocalCommit(LocalCommit),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ClientTimeout {
    pub req_id: u32,
}
impl ClientTimeout {
    pub fn new(req_id: u32) -> Self {
        ClientTimeout { req_id }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ClientRequest {
    pub operation: u32,
    pub sender_id: u32,
}
impl ClientRequest {
    pub fn new(operation: u32, sender_id: u32) -> Self {
        ClientRequest {
            operation,
            sender_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct OrderRequest {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32,
}
impl OrderRequest {
    pub fn new(c_req: ClientRequest, view: u64, seq_number: u64, sender_id: u32) -> Self {
        OrderRequest {
            c_req,
            view,
            seq_number,
            sender_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct SpeculativeResponse {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32,
}
impl SpeculativeResponse {
    pub fn new(c_req: ClientRequest, view: u64, seq_number: u64, sender_id: u32) -> Self {
        SpeculativeResponse {
            c_req,
            view,
            seq_number,
            sender_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Commit {
    pub req_id: u32,
    pub certificate: Vec<SpeculativeResponse>,
    pub sender_id: u32,
}
impl Commit {
    pub fn new(req_id: u32, certificate: Vec<SpeculativeResponse>, sender_id: u32) -> Self {
        Commit {
            req_id,
            certificate,
            sender_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct LocalCommit {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32,
}
impl LocalCommit {
    pub fn new(c_req: ClientRequest, view: u64, seq_number: u64, sender_id: u32) -> Self {
        LocalCommit {
            c_req,
            view,
            seq_number,
            sender_id,
        }
    }
}
