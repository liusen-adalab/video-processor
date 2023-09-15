use crate::{
    controller::worker::JobOutput,
    manager::{manager, JobId},
};

use anyhow::Result;

pub struct WorkerRegisterDto {}

pub async fn new_worker(worker: WorkerRegisterDto) {
    let manager = manager();
    manager.worker_register(todo!());
    todo!()
}

pub async fn pause_worker() {
    todo!()
}

pub async fn resume_worker() {
    todo!()
}

pub async fn kill_worker() {
    todo!()
}

pub async fn start_worker() {
    todo!()
}

// idempotent
pub async fn job_done(worker_id: i64, job_id: JobId, res: JobOutput) -> Result<()> {
    // pg tx:
    // ?save to db
    // manager handle
    //
    // ?response to worker
    todo!()
}

// idempotent
pub async fn job_failed(worker_id: i64, job_id: i64) {
    todo!()
}

// idempotent
pub async fn job_progress(worker_id: i64, job_id: i64, progress: f64) {
    todo!()
}
