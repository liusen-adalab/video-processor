use std::{collections::HashMap, path::PathBuf};

use utils::id_new_type;

id_new_type!(JobId);
id_new_type!(TaskId);

pub trait Job {
    type Output;

    fn id(&self) -> JobId;

    fn task_id(&self) -> TaskId;

    fn output(&self) -> Self::Output;
}

pub trait Task {
    type InitJob: Job;

    // fn job_done<O>(
    //     &mut self,
    //     job: Box<dyn Job<Output = O>>,
    //     output: O,
    // ) -> Option<Vec<Box<dyn Job<Output = O>>>>;

    fn init_job(&self) -> Self::InitJob;

    fn is_done(&self) -> bool;
}

pub struct Manager {
    tasks: HashMap<TaskId, Box<dyn Task<InitJob = DemuxJob>>>,
}

impl Manager {
    pub fn job_done<T>(&mut self, job: Box<dyn Job<Output = T>>) {
        todo!()
    }
}

pub struct TranscodeTask {
    id: TaskId,
    progress: Progress,
}

#[derive(Clone)]
pub struct Progress {
    pub total: usize,
    pub current: usize,
}

pub struct ParseJob {
    id: JobId,
    src: PathBuf,
    params: ParseParams,
}

pub struct DemuxJob {
    id: JobId,
    src: PathBuf,
    params: DemuxParams,
}

pub struct DemuxParams {}

pub struct DemuxOutput {
    audio: PathBuf,
}

impl Job for DemuxJob {
    type Output = DemuxOutput;

    fn id(&self) -> JobId {
        todo!()
    }

    fn task_id(&self) -> TaskId {
        todo!()
    }

    fn output(&self) -> Self::Output {
        todo!()
    }
}

pub struct ParseParams {}

pub struct ThumbnailJob {
    id: JobId,
    params: ThumbnailParams,
}

pub struct ThumbnailParams {}
