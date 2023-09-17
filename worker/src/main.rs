use std::path::PathBuf;

use futures::SinkExt;
use tokio::net::TcpStream;
use utils::{codec, macros::codec::tokio_util::codec::Framed};

fn main() {
    println!("Hello, world!");
}

pub enum WorkerType {
    Parse,
}

fn start(worker_type: WorkerType) {
    match worker_type {
        WorkerType::Parse => {
            // do something
        }
    }
}

#[derive(Serialize)]
pub struct JobRaw {
    job: Vec<u8>,
}

#[derive(serde::Deserialize)]
pub struct ManagerMsg<J> {
    id: i64,
    msg: ManagerCmd<J>,
}

#[derive(serde::Deserialize)]
pub enum ManagerCmd<J> {
    Stop,
    Pause,
    Resume,
    CancelTask(i64),
    Job(J),
}

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub enum WorkerMsg<R> {
    JobReport(R),
    Heartbeat,
    SystemInfo(SystemInfo),
}

#[derive(Serialize)]
pub struct SystemInfo {}

codec!(WorkerCodec, encode: WorkerMsg<R>, decode: ManagerMsg<J>);

pub trait Worker<J>
where
    J: VideoJob<Worker = Self>,
{
    fn new_job(job: J);

    fn run(self);
}

pub trait VideoJob: Sized {
    type Worker: Worker<Self>;
}

#[derive(Deserialize)]
pub struct ParseJob {
    path: PathBuf,
}

pub struct ParseWorker {}

async fn aa() {
    use futures::stream::StreamExt;
    let tcp = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let mut stream = Framed::new(tcp, WorkerCodec::<ParseJob>::new());
    let msg = stream.next().await;
    stream
        .send(WorkerMsg::JobReport(JobRaw { job: vec![1, 2, 3] }))
        .await
        .unwrap();
    let msg = stream.next().await.unwrap();
}

impl Worker<ParseJob> for ParseWorker {
    fn new_job(job: ParseJob) {
        todo!()
    }

    fn run(self) {
        todo!()
    }
}

impl VideoJob for ParseJob {
    type Worker = ParseWorker;
}
