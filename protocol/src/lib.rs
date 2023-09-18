use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub mod handshake;
pub mod manager_msg;
pub mod parse;
pub mod worker_msg;

pub type TaskId = i64;
pub type JobId = i64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum JobType {
    Parse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemInfo {
    pub cpu: f64,
    pub mem: f64,
}

pub trait Job: Serialize + DeserializeOwned + Send + Sync + 'static {
    fn ty() -> JobType;
}
