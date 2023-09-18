use std::path::PathBuf;

use derive_more::From;
use serde::{Deserialize, Serialize};

use crate::Job;

#[derive(Debug, Deserialize, Serialize)]
pub struct ParseJob {
    pub task_id: i64,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone, From)]
pub struct ParseOutput(pub Option<String>);

impl Job for ParseJob {
    fn ty() -> crate::JobType {
        crate::JobType::Parse
    }
}
