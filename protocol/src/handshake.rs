use serde::{Deserialize, Serialize};

use crate::{JobType, SystemInfo};

#[derive(Deserialize, Serialize)]
pub enum HandShakeProtocol {
    Req(HandShakeReq),
    Resp(HandShakeResp),
}

#[derive(Deserialize, Serialize)]
pub struct HandShakeReq {
    pub worker_type: JobType,
    pub pid: u32,
    pub system: SystemInfo,
}

#[derive(Deserialize, Serialize)]
pub struct HandShakeResp {
    pub accepted: bool,
    pub msg: Option<String>,
}
