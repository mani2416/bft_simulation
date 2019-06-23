#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ZyzzyvaMessage {
    ClientRequest(ClientRequest),
    OrderRequest(OrderRequest),
    SpeculativeResponse(SpeculativeResponse),
    Commit(Commit)
}


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ClientRequest {
    pub operation: u32,
    pub sender_id: u32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct OrderRequest {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct SpeculativeResponse {
    pub c_req: ClientRequest,
    pub view: u64,
    pub seq_number: u64,
    pub sender_id: u32,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Commit {
    pub certificate: Vec<SpeculativeResponse>,
    pub sender_id: u32,
}