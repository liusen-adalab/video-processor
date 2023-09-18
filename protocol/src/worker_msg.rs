use serde::{Deserialize, Serialize};

use crate::{JobId, JobType, SystemInfo, TaskId};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WorkerMsg<O> {
    JobEvent(JobEventWraper<O>),
    SystemInfo(SystemInfo),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobEventWraper<O> {
    pub task_id: TaskId,
    pub job_id: JobId,
    pub job_type: JobType,
    pub event: JobEvent<O>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum JobEvent<O> {
    Ok(O),
    Err(String),
    Progress(Progress),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Progress {
    total: u64,
    current: u64,
}
