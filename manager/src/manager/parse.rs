use std::path::PathBuf;

use serde::Serialize;

use crate::manager::Progress;

use super::{Job, JobEvent, JobRaw, Task, TaskId, TaskRaw, TaskResult, TaskStatus};

pub struct ParseTask {
    id: TaskId,
    status: TaskStatus,
    params: ParseParams,
}

pub struct ParseParams {
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct ParseJob {
    path: PathBuf,
}

pub struct ParseJobEvent {
    task_id: TaskId,
    output: Result<String, String>,
}

impl JobEvent for ParseJobEvent {
    type Task = ParseTask;

    type Job = ParseJob;

    fn job_id(&self) -> i64 {
        0
    }

    fn task_id(&self) -> TaskId {
        self.task_id
    }
}

impl Task for ParseTask {
    type Job = ParseJob;
    type Event = ParseJobEvent;

    fn id(&self) -> TaskId {
        self.id
    }

    fn status(&self) -> &TaskStatus {
        &self.status
    }

    fn result(&self) -> Option<TaskResult> {
        match &self.status {
            crate::manager::TaskStatus::Done(output) => Some(TaskResult::Success {
                id: self.id,
                output: Some(output.clone()),
            }),
            crate::manager::TaskStatus::Failed(reason) => Some(TaskResult::Failure {
                id: self.id,
                reason: reason.clone(),
            }),
            _ => None,
        }
    }

    fn handle_event(&mut self, event: Self::Event) {
        match event.output {
            Ok(output) => {
                self.status = TaskStatus::Done(output);
            }
            Err(reason) => {
                self.status = TaskStatus::Failed(reason);
            }
        }
    }

    fn next_job(&mut self) -> Option<Self::Job> {
        match self.status {
            crate::manager::TaskStatus::Created => {
                self.status = crate::manager::TaskStatus::Running(Progress::new(1));
                let job = ParseJob {
                    path: self.params.path.clone(),
                };
                Some(job)
            }
            _ => None,
        }
    }

    fn have_next_job(&self) -> bool {
        matches!(self.status, crate::manager::TaskStatus::Created)
    }
}

impl Job for ParseJob {
    type Task = ParseTask;

    fn worker_type(&self) -> super::WorkerType {
        super::WorkerType::Parse
    }
}

impl TryFrom<TaskRaw> for ParseTask {
    type Error = ();

    fn try_from(value: TaskRaw) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<JobRaw> for ParseJob {
    type Error = ();

    fn try_from(value: JobRaw) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl From<ParseJob> for JobRaw {
    fn from(value: ParseJob) -> Self {
        todo!()
    }
}
