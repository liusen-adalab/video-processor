use std::{collections::HashMap, path::PathBuf, sync::Arc};

use parking_lot::RwLock;
use utils::id_new_type;

mod aa;
mod bb;
mod cc;

#[derive(Clone)]
pub struct Manager {
    task_res_subscriber: tokio::sync::mpsc::UnboundedSender<TaskEvent>,
    tasks: HashMap<TaskId, Arc<RwLock<Task>>>,
    workers: Arc<RwLock<HashMap<WorkerId, Worker>>>,
}

struct ManagerInner {
    tasks: HashMap<TaskId, Task>,
    workers: HashMap<WorkerId, Worker>,
}

pub fn manager() -> &'static Manager {
    todo!()
}

id_new_type!(TaskId);

pub struct Task {
    id: TaskId,
    progress: Progress,
    inner: TaskInner,
}

pub enum TaskInner {
    Parse(ParseTask),
    Thumbnail(ThumbnailTask),
    Transcode(TranscodeTask),
}

pub struct TranscodeTask {
    id: i64,
    path: PathBuf,
}

pub struct ParseTask {
    id: i64,
    path: PathBuf,
}

pub struct ThumbnailTask {
    id: i64,
    path: PathBuf,
    dst_dir: PathBuf,
}

pub struct TranscodeJob {
    task_id: i64,
    path: PathBuf,
}

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct ProcessedWorks: i16 {
        const Origin         = 0;
        const Demuxed        = 1 << 1;
        const Splited        = 1 << 2;
        const H264Done       = 1 << 3;
        const AudioDone      = 1 << 4;
        const Muxed          = 1 << 5;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TaskStep {
    /// 已转为 264 编码
    H264Converted,
    /// 已分流存储
    Demuxed,
    /// 已切片
    Splited,
}
id_new_type!(WorkerId);

type WorkerAddress = tokio::sync::mpsc::UnboundedSender<Arc<WorkerJob>>;

pub struct Worker {
    id: WorkerId,
    mailbox: WorkerAddress,
    info: WorkerInfo,
    status: WorkerStatus,
}

pub struct WorkerInfo {
    ip: String,
    port: u16,
    process_id: i64,
    w_type: WorkerType,
}

pub enum WorkerStatus {
    /// 空闲
    Idle,
    /// 工作中
    Working { jobs: HashMap<JobId, WorkerJob> },
    /// 死亡
    Died,
}

id_new_type!(JobId);

pub struct WorkerJob {
    id: JobId,
    task: Arc<RwLock<Task>>,
    params: WorkerJobParams,
    progress: Progress,
}

impl WorkerJob {
    fn coefficient(&self) -> f64 {
        todo!()
    }
}

pub enum WorkerJobParams {
    Parse(ParseParams),
    Thumbnail(ThumbnailParams),
}

pub struct ThumbnailParams {}

pub struct ParseParams {
    path: PathBuf,
}

pub enum WorkerType {
    Parse,
    Thumbnail,
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use utils::id_macro::{self, derive_more};

use crate::controller::worker::{JobOutput, ParseOutput};

// outter event
impl Manager {
    pub fn new_task(&self, task: Task) {
        // dispatch
        todo!()
    }

    pub fn job_done(&self, worker_id: WorkerId, job_id: JobId, res: JobOutput) {
        // move job to done
        match res {
            JobOutput::Parse(parse) => {
                let mut workers = self.workers.write();
                if let Some(worker) = workers.get_mut(&worker_id) {
                    match &mut worker.status {
                        WorkerStatus::Idle => {
                            info!(%worker_id, "received event from idle worker");
                        }
                        WorkerStatus::Working { jobs } => {
                            if let Some(job) = jobs.remove(&job_id) {
                                let task = job.task.read();
                                let task_id = task.id;
                                self.task_done(task_id, TaskOutput::Parse(parse)).unwrap();
                            }
                        }
                        WorkerStatus::Died => {
                            info!(%worker_id, "received event from dead worker");
                        }
                    }
                }
            }
            JobOutput::Thumbnail => {
                let mut workers = self.workers.write();
                if let Some(worker) = workers.get_mut(&worker_id) {
                    match &mut worker.status {
                        WorkerStatus::Idle => {
                            info!(%worker_id, "received event from idle worker");
                        }
                        WorkerStatus::Working { jobs } => {
                            if let Some(job) = jobs.remove(&job_id) {
                                let task_id = job.task.read().id;
                                self.task_done(task_id, TaskOutput::Thumbnail).unwrap();
                            }
                        }
                        WorkerStatus::Died => {
                            info!(%worker_id, "received event from dead worker");
                        }
                    }
                }
            }
        }
        todo!()
    }

    pub fn job_failed(&self, worker_id: WorkerId, job_id: JobId) {
        // move job to failed
        todo!()
    }

    pub fn job_progress(&self, worker_id: WorkerId, job_id: JobId, progress: Progress) {
        // update job progress
        let mut workers = self.workers.write();
        if let Some(worker) = workers.get_mut(&worker_id) {
            match &mut worker.status {
                WorkerStatus::Idle => {
                    info!(%worker_id, "received event from idle worker");
                }
                WorkerStatus::Working { jobs } => {
                    if let Some(job) = jobs.get_mut(&job_id) {
                        job.progress = progress;

                        // let mut tasks = self.tasks.write();
                        // if let Some(task) = tasks.get_mut(&job.task_id) {
                        //     if job.progress.finished >= job.progress.total {
                        //         todo!()
                        //     }
                        // }
                    } else {
                        warn!(%worker_id, "no such worker");
                    }
                }
                WorkerStatus::Died => {
                    info!(%worker_id, "received event from dead worker");
                }
            }
        }
        todo!()
    }

    pub fn worker_died(&self, worker_id: i64) {
        // move worker to died
        todo!()
    }

    pub fn task_ended(&self, task_id: i64) {
        // move task to ended
        todo!()
    }
}

pub struct Progress {
    total: u32,
    finished: u32,
}

pub struct TaskEvent {
    task_id: TaskId,
    msg: TaskEventMsg,
}

pub enum TaskEventMsg {
    Failed,
    Done(TaskOutput),
}

use derive_more::From;

#[derive(From)]
pub enum TaskOutput {
    Parse(ParseOutput),
    Thumbnail,
}

// publish
impl Manager {
    pub fn subcribe_task_status(&self) {
        todo!()
    }

    pub fn subcribe_job(&self) {
        todo!()
    }

    fn task_done(&self, task_id: TaskId, task_output: TaskOutput) -> Result<()> {
        // self.task_res_subscriber
        //     .send(TaskEvent {
        //         task_id,
        //         msg: TaskEventMsg::Done(task_output),
        //     })
        //     .unwrap();
        // let mut tasks = self.tasks.write();
        // tasks.remove(&task_id);
        todo!()
    }

    fn task_failed(&self, task_id: i64) -> Result<()> {
        todo!()
    }
}

// command
impl Manager {
    pub fn worker_register(&self, worker: Worker) {
        todo!()
    }

    pub fn pause_worker(&self, worker_id: i64) {
        todo!()
    }

    pub fn resume_worker(&self, worker_id: i64) {
        todo!()
    }

    pub fn kill_worker(&self, worker_id: i64) {
        todo!()
    }

    pub fn dispatch(&self) -> Vec<Task> {
        todo!()
    }

    pub fn pause_task(&self, task_id: i64) {
        todo!()
    }

    pub fn resume_task(&self, task_id: i64) {
        todo!()
    }

    pub fn cancel_task(&self, task_id: i64) {
        todo!()
    }
}

// query
impl Manager {
    pub fn get_task(&self, task_id: i64) -> Option<Task> {
        todo!()
    }

    pub fn get_worker(&self, worker_id: i64) -> Option<Worker> {
        todo!()
    }

    pub fn get_workers(&self) -> Vec<Worker> {
        todo!()
    }

    pub fn get_tasks(&self) -> Vec<Task> {
        todo!()
    }

    pub fn sync_status(&self) {
        todo!()
    }
}

pub struct WorkerEvent {
    worker_id: i64,
    task_id: i64,
    msg: WorkerEventMsg,
}

pub enum WorkerEventMsg {
    /// 任务完成
    TaskDone,
    /// 任务失败
    TaskFailed { reason: String },
    /// 任务进度
    TaskProgress { step: TaskStep },
}
