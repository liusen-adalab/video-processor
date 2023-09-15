use crate::{
    application,
    manager::{JobId, Task},
};

pub async fn start_listener() {
    todo!()
}

pub struct Worker {
    id: i64,
}

impl Worker {
    async fn new_job(&self) {
        todo!()
    }

    async fn handle_event(&self, event: WorkerEvent) -> anyhow::Result<()> {
        match event.msg {
            WorkerEventMsg::JobDone(result) => {
                let res = application::worker::job_done(self.id, event.job_id, result).await;
                if res.is_ok() {
                    // ack worker event
                }
            }
            WorkerEventMsg::JobFailed { reason } => todo!(),
            WorkerEventMsg::JobProgress { progress } => todo!(),
        }
        todo!()
    }

    async fn job_done(&self, task: Task) {
        todo!()
    }
}

pub struct Job {
    id: i64,
    task_id: i64,
}

pub struct WorkerEvent {
    worker_id: i64,
    job_id: JobId,
    msg: WorkerEventMsg,
}

pub enum WorkerEventMsg {
    JobDone(JobOutput),
    JobFailed { reason: String },
    JobProgress { progress: f64 },
}

pub enum JobOutput {
    Parse(ParseOutput),
    Thumbnail,
}

pub struct ParseOutput {}
