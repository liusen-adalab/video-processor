use crate::{application, manager::Task};

pub async fn start_subscriber() {
    todo!()
}

async fn new_task(task: Task) {
    let res = application::task::new_task(task).await;
    if res.is_ok() {
        // ack
    }
}
