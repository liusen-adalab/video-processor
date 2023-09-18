use std::path::PathBuf;

use protocol::{
    manager_msg::IntoRaw,
    parse::{ParseJob, ParseOutput},
    worker_msg::JobEventWraper,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{JobEventHandler, JobOutput, JobRaw, Task, TaskId, TaskResult, TaskStatus};

#[derive(Serialize, Deserialize)]
pub struct ParseTask {
    id: TaskId,
    status: TaskStatus,
    params: ParseParams,
}

#[derive(Serialize, Deserialize)]
pub struct ParseParams {
    pub path: PathBuf,
}

impl Task for ParseTask {
    fn id(&self) -> TaskId {
        self.id
    }

    fn status(&self) -> &TaskStatus {
        &self.status
    }

    fn result(&self) -> Option<TaskResult> {
        match &self.status {
            TaskStatus::Created => None,
            TaskStatus::Running(_) => None,
            TaskStatus::Done(out) => Some(TaskResult::Success {
                id: self.id,
                output: Some(out.to_string()),
            }),
            TaskStatus::Failed(reason) => Some(TaskResult::Failure {
                id: self.id,
                reason: reason.clone(),
            }),
        }
    }

    fn next_job(&mut self) -> Option<JobRaw> {
        if let TaskStatus::Created = self.status {
            let job = ParseJob {
                task_id: self.id,
                path: self.params.path.clone(),
            }
            .into_raw();
            Some(job)
        } else {
            None
        }
    }

    fn have_next_job(&self) -> bool {
        matches!(self.status, TaskStatus::Created)
    }

    fn ty() -> super::TaskType {
        super::TaskType::Parse
    }
}

impl JobEventHandler<ParseOutput> for ParseTask {
    fn handle_event(&mut self, event: JobEventWraper<ParseOutput>) {
        match event.event {
            protocol::worker_msg::JobEvent::Ok(out) => {
                self.status = TaskStatus::Done("todo".to_string())
            }
            protocol::worker_msg::JobEvent::Err(err) => self.status = TaskStatus::Failed(err),
            protocol::worker_msg::JobEvent::Progress(p) => {
                warn!(?p, "progress");
                self.status = TaskStatus::Failed("should not occur progress".to_string())
            }
        }
    }
}

impl JobOutput for ParseOutput {
    type Task = ParseTask;
}
