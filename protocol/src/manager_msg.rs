use serde::{Deserialize, Serialize};

use crate::{Job, JobType, TaskId};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManagerMsg {
    pub cmd: ManagerCmd,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ManagerCmd {
    CancelTask(TaskId),
    Job(JobRaw),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobRaw {
    pub ty: JobType,
    pub payload: Vec<u8>,
}

pub trait TryFromRaw: Sized {
    fn try_from(raw: JobRaw) -> Result<Self, anyhow::Error>;
}

impl<T> TryFromRaw for T
where
    T: Job,
{
    fn try_from(raw: JobRaw) -> Result<Self, anyhow::Error> {
        Ok(bincode::deserialize(&raw.payload)?)
    }
}

pub trait IntoRaw: Sized {
    fn into_raw(&self) -> JobRaw;
}

impl<T> IntoRaw for T
where
    T: Job,
{
    fn into_raw(&self) -> JobRaw {
        let p = bincode::serialize(&self).unwrap();
        JobRaw {
            ty: Self::ty(),
            payload: p,
        }
    }
}
