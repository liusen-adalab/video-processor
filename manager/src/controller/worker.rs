use std::marker::PhantomData;

use serde::Serialize;

use crate::manager::{Job, JobRaw};

pub struct Worker<T: Job> {
    phamtom: PhantomData<T>,
    mailbox: tokio::sync::mpsc::UnboundedReceiver<JobRaw>,
}

#[derive(Serialize)]
pub struct VideoCmd<J> {
    id: i64,
    msg: Msg<J>,
}

#[derive(Serialize)]
pub enum Msg<J> {
    Stop,
    Job(J),
}

impl<J, E> Worker<J>
where
    J: Job,
    J: TryFrom<JobRaw, Error = E>,
    E: std::fmt::Debug,
{
    async fn run(&mut self) {
        while let Some(job) = self.mailbox.recv().await {
            let job = J::try_from(job).unwrap();
            let msg = serde_json::to_string(&Msg::Job(job));

            todo!()
        }
    }
}

pub struct JobDto {}
