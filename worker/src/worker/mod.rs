use std::time::Duration;

use anyhow::{Context, Result};
use futures::{stream::FuturesUnordered, Future, SinkExt, Stream, StreamExt};
use pin_project_lite::pin_project;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::{net::TcpStream, select, sync::oneshot, time};
use tracing::error;
use utils::{
    codec,
    macros::codec::tokio_util::{self},
};

pub mod parse;

#[derive(Serialize, Deserialize)]
pub struct WorkerCommonCfg {
    ty: WorkerType,
    manager_url: String,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WorkerType {
    Parse,
}

pub fn start(cfg: &'static WorkerCommonCfg) {
    match cfg.ty {
        WorkerType::Parse => {}
    }
}

fn run_bg<W>(cfg: &'static WorkerCommonCfg)
where
    W: Worker,
    <W::Handler as Future>::Output: Send,
{
    tokio::spawn(async move {
        loop {
            let tracker = WorkerTracker::<W>::new(cfg).await;
            if let Err(err) = tracker.run().await {
                error!("Worker error: {:?}", err);
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
    todo!()
}

struct WorkerTracker<W: Worker> {
    state: State,
    conn: Connection<W::Job>,
    job_count: usize,
    jobs: FuturesUnordered<W::Handler>,
}

impl<W: Worker> Drop for WorkerTracker<W> {
    fn drop(&mut self) {
        todo!()
    }
}

enum State {
    Paused,
    Running,
}

impl<W: Worker> WorkerTracker<W> {
    async fn new(cfg: &WorkerCommonCfg) -> Self {
        let tracker = WorkerTracker::<W> {
            state: State::Running,
            conn: Connection::connect_until_success(&cfg.manager_url).await,
            jobs: Default::default(),
            job_count: 0,
        };
        tracker
    }

    async fn run(mut self) -> Result<()> {
        let mut report_interval = time::interval(Duration::from_secs(5));
        loop {
            select! {
                msg = self.conn.next() => {
                    self.handle_msg(msg).await?;
                }
                _ = report_interval.tick() => {
                    self.report_info().await?;
                }
                Some(output) = self.jobs.next() => {
                    self.report_ouput(output).await?;
                }
            }
        }
    }

    async fn report_ouput(&mut self, output: <W::Handler as Future>::Output) -> Result<()> {
        self.conn
            .send(WorkerMsg::JobReport(output))
            .await
            .context("send job output")?;
        todo!()
    }

    async fn report_info(&mut self) -> Result<()> {
        let info = SystemInfo { cpu: 1.0, mem: 2.0 };
        self.conn
            .send(WorkerMsg::<()>::SystemInfo(info))
            .await
            .context("send system info")?;
        Ok(())
    }

    async fn handle_msg(&mut self, msg: ManagerMsg<W::Job>) -> Result<()> {
        match msg.cmd {
            ManagerCmd::Pause => self.state = State::Paused,
            ManagerCmd::Resume => self.state = State::Running,
            ManagerCmd::CancelTask(id) => {
                self.cancel_task(id).await;
            }
            ManagerCmd::Job(job) => {
                self.job_count += 1;
                self.jobs.push(W::run(job));
            }
        }
        Ok(())
    }

    async fn cancel_task(&mut self, id: TaskId) {
        let jobs = std::mem::replace(&mut self.jobs, FuturesUnordered::new());
        self.jobs = jobs
            .into_iter()
            .filter_map(|j| {
                if j.task_id() == id {
                    j.stop();
                    self.job_count -= 1;
                    None
                } else {
                    Some(j)
                }
            })
            .collect();
    }
}

type Framed<J> = tokio_util::codec::Framed<TcpStream, WorkerCodec<J>>;

struct Connection<Job> {
    url: String,
    state: ConnectionState,
    conn: Framed<Job>,
}

enum ConnectionState {
    Connected,
    Disconnected,
}

impl<J> Connection<J> {
    async fn send<M: Serialize>(&mut self, msg: WorkerMsg<M>) -> Result<()> {
        self.conn.send(msg).await?;
        Ok(())
    }
}

impl<J> Connection<J>
where
    J: DeserializeOwned,
{
    async fn connect_until_success(url: &str) -> Self {
        let framed = Self::framed_loop(url).await;
        Self {
            state: ConnectionState::Connected,
            conn: framed,
            url: url.to_string(),
        }
    }

    async fn framed_loop(url: &str) -> Framed<J> {
        let mut wait = Duration::from_secs(1);
        let max = Duration::from_secs(20);

        loop {
            match Self::framed(url).await {
                Ok(f) => break f,
                Err(err) => {
                    error!("Connection error: {}", err);
                    time::sleep(wait).await;
                    wait += Duration::from_secs(2);
                    wait = wait.max(max);
                }
            }
        }
    }

    async fn framed(url: &str) -> Result<Framed<J>> {
        let tcp = TcpStream::connect(url).await?;
        let framed = Framed::new(tcp, WorkerCodec::new());
        Ok(framed)
    }

    async fn next(&mut self) -> ManagerMsg<J> {
        loop {
            let msg = self.conn.next().await;
            match msg {
                Some(Ok(msg)) => break msg,
                Some(Err(err)) => {
                    self.state = ConnectionState::Disconnected;
                    error!("Error: {}", err);
                }
                None => {
                    self.state = ConnectionState::Disconnected;
                    error!(%self.url, "Connection closed");
                }
            }

            self.conn = Self::framed_loop(&self.url).await;
        }
    }
}

#[derive(Deserialize)]
pub struct ManagerMsg<J> {
    cmd: ManagerCmd<J>,
}

type TaskId = i64;

#[derive(Deserialize)]
pub enum ManagerCmd<J> {
    Pause,
    Resume,
    CancelTask(TaskId),
    Job(J),
}

#[derive(Serialize)]
pub enum WorkerMsg<R = ()> {
    JobReport(R),
    Heartbeat,
    SystemInfo(SystemInfo),
}

impl<R> From<SystemInfo> for WorkerMsg<R> {
    fn from(value: SystemInfo) -> Self {
        WorkerMsg::SystemInfo(value)
    }
}

#[derive(Serialize)]
pub struct SystemInfo {
    cpu: f64,
    mem: f64,
}

codec!(WorkerCodec, encode: WorkerMsg<R>, decode: ManagerMsg<J>);

trait Worker {
    type Job: Job<Worker = Self>;
    type Output: Send + 'static + Serialize;
    type Handler: WorkerHandler<Output = Self::Output>;

    fn run(job: Self::Job) -> Self::Handler;
}

trait Job: Sized + DeserializeOwned + Send + Sync + 'static {
    type Worker: Worker<Job = Self>;

    fn task_id(&self) -> TaskId;
}

trait WorkerHandler: 'static + Send + Sync + Unpin + Future {
    type EventStream: Stream<Item = JobEvent>;

    fn task_id(&self) -> TaskId;

    fn stop(&self);

    fn event_stream(&self) -> Option<Self::EventStream>;
}

pub struct JobEvent {
    pub task_id: TaskId,
}

pin_project! {
    pub struct CommonJobHandler<T> {
        #[pin]
        resp: oneshot::Receiver<T>,
        handle: oneshot::Sender<()>,
    }
}

impl<T> CommonJobHandler<T> {
    fn new() -> Self {
        todo!()
    }
}

impl<T> Future for CommonJobHandler<T> {
    type Output = T;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();

        this.resp.poll(cx).map(|r| r.unwrap())
    }
}

async fn bb() {}

fn aa() {
    let mut a = FuturesUnordered::new();
    a.push(bb());
    a.push(bb());

    let b = FuturesUnordered::new();
    b.push(bb());
}
