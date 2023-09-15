use std::path::PathBuf;

use crate::{
    infrastructure::repo_task,
    manager::{manager, Task},
};
use anyhow::Result;
use utils::{
    db_pools::postgres::{pg_conn, PgConn},
    pg_tx,
};

pub async fn new_task(task: Task) -> Result<()> {
    let conn = &mut pg_conn().await?;
    // let res = pg_tx!(new_task_tx, task);
    repo_task::save(&task, conn).await?;
    tick(task);

    Ok(())
}

async fn new_task_tx(task: Task, conn: &mut PgConn) -> Result<()> {
    repo_task::save(&task, conn).await?;
    todo!()
}

fn tick(task: Task) {
    let manager = manager();
    manager.new_task(task);
    let jobs = manager.dispatch();
    for job in jobs {
        todo!()
    }
    todo!()
}

pub async fn pause_task() {
    todo!()
}

pub async fn resume_task() {
    todo!()
}

pub async fn cancel_task() {
    todo!()
}

// pub struct WorkerEvent {
//     worker_id: i64,
//     task_id: i64,
//     msg: WorkerEventMsg,
// }

// pub enum WorkerEventMsg {
//     /// 任务完成
//     TaskDone,
//     /// 任务失败
//     TaskFailed { reason: String },
//     /// 任务进度
//     TaskProgress { progress: f64 },
// }

// pub async fn worker_report(report: WorkerEvent) -> Result<()> {
//     let manager = manager();
//     match report.msg {
//         WorkerEventMsg::TaskDone => {
//             // pg tx:
//             // ?save to db
//             // ?send mq
//             // remove task from worker
//             //
//             // ?response to worker
//         }
//         WorkerEventMsg::TaskFailed { reason } => {
//             // save task
//             //
//         }
//         WorkerEventMsg::TaskProgress { progress } => todo!(),
//     }
//     todo!()
// }
