use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Context, Result};
use futures::{SinkExt, StreamExt};
use protocol::{
    handshake::{HandShakeProtocol, HandShakeReq},
    manager_msg::{ManagerCmd, ManagerMsg},
    worker_msg::{JobEvent, JobEventWraper, WorkerMsg},
    JobType, SystemInfo, TaskId,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{net::TcpStream, select, sync::mpsc, time};
use tracing::{error, info, warn};
use utils::{
    codec,
    macros::codec::{
        bincode,
        tokio_util::{self, codec::Decoder},
    },
};

use self::parse::ParseWorker;

mod parse;

pub struct WorkerCommonCfg {
    pub worker_type: JobType,
    pub url: String,
}

pub fn start(cfg: &'static WorkerCommonCfg) {
    match cfg.worker_type {
        JobType::Parse => {
            WorkerRouter::<ParseWorker>::run_bg(cfg);
        }
    }
}

trait Worker: 'static + Send {
    type Job: Job;
    type Output: Serialize + Send + 'static;
    type Handler: WorkerHandler;

    fn worker_type() -> JobType;

    fn spawn(
        job: Self::Job,
        chan: mpsc::UnboundedSender<JobEventWraper<Self::Output>>,
    ) -> Self::Handler;
}

trait Job: DeserializeOwned + Send + 'static {
    fn task_id(&self) -> TaskId;
}

trait WorkerHandler: Send + 'static {
    fn cancel(self);
}

codec!(WorkerCodec, encode: WorkerMsg<E>, decode: ManagerMsg);

type Framed = tokio_util::codec::Framed<TcpStream, WorkerCodec>;

struct Connection<W: Worker> {
    url: String,
    conn: Framed,
    phamtom: std::marker::PhantomData<W>,
}

struct WorkerRouter<W: Worker> {
    conn: Connection<W>,
    job_count: usize,
    jobs: HashMap<TaskId, Vec<W::Handler>>,
    recv: mpsc::UnboundedReceiver<JobEventWraper<W::Output>>,
    send: mpsc::UnboundedSender<JobEventWraper<W::Output>>,
}

impl<W> WorkerRouter<W>
where
    W: Worker + 'static,
{
    async fn new(cfg: &'static WorkerCommonCfg) -> Self {
        let conn = Connection::connect_until_success(&cfg.url).await;
        let (send, recv) = mpsc::unbounded_channel();
        Self {
            conn,
            job_count: Default::default(),
            jobs: Default::default(),
            recv,
            send,
        }
    }

    fn run_bg(cfg: &'static WorkerCommonCfg) {
        tokio::spawn(async move {
            let mut router = WorkerRouter::<W>::new(cfg).await;
            loop {
                if let Err(err) = router.run().await {
                    error!("router error: {}", err);
                    router.conn = Connection::connect_until_success(&cfg.url).await;
                }
            }
        });
    }

    async fn run(&mut self) -> Result<()> {
        let mut report_interval = time::interval(Duration::from_secs(5));
        loop {
            select! {
                msg = self.conn.next() => {
                    self.handle_msg(msg).await?;
                }
                _ = report_interval.tick() => {
                    self.report_info().await?;
                }
                Some(event) = self.recv.recv() => {
                    self.handler_worker_event(event).await?;
                }
            }
        }
    }

    async fn handler_worker_event(&mut self, event: JobEventWraper<W::Output>) -> Result<()> {
        if matches!(event.event, JobEvent::Ok(_) | JobEvent::Err(_)) {
            self.job_count -= 1;
        }
        self.conn.send(WorkerMsg::JobEvent(event)).await?;
        Ok(())
    }

    async fn handle_msg(&mut self, msg: ManagerMsg) -> Result<()> {
        match msg.cmd {
            ManagerCmd::CancelTask(id) => {
                self.cancel_task(id).await?;
            }
            ManagerCmd::Job(job) => {
                let job =
                    bincode::deserialize::<W::Job>(&job.payload).context("job type not match")?;
                self.job_count += 1;
                let task_id = job.task_id();
                let h = W::spawn(job, self.send.clone());
                self.jobs.entry(task_id).or_default().push(h);
            }
        }
        Ok(())
    }

    async fn cancel_task(&mut self, id: TaskId) -> Result<()> {
        if let Some(handlers) = self.jobs.remove(&id) {
            for h in handlers {
                h.cancel();
            }
        } else {
            warn!("Task {} not found", id);
        }
        Ok(())
    }

    async fn report_info(&mut self) -> Result<()> {
        let info = SystemInfo { cpu: 1.0, mem: 2.0 };
        self.conn
            .send(WorkerMsg::<()>::SystemInfo(info))
            .await
            .context("send system info")?;
        Ok(())
    }
}

codec!(HandShakeCodec, encode: HandShakeProtocol, decode: HandShakeProtocol);

impl<W: Worker> Connection<W> {
    async fn connect_until_success(url: &str) -> Self {
        let framed = Self::framed_loop(url).await;
        Self {
            conn: framed,
            url: url.to_string(),
            phamtom: std::marker::PhantomData,
        }
    }

    async fn handshake(stream: TcpStream) -> Result<Framed> {
        info!("performing handshake");
        let mut framed = HandShakeCodec::new().framed(stream);
        framed
            .send(HandShakeProtocol::Req(HandShakeReq {
                worker_type: W::worker_type(),
                pid: 1,
                system: SystemInfo { cpu: 1.0, mem: 1.0 },
            }))
            .await
            .context("send handshake request")?;

        match framed.next().await {
            Some(Ok(resp)) => match resp {
                HandShakeProtocol::Resp(resp) => {
                    if resp.accepted {
                        info!("Handshake success");
                        let framed = WorkerCodec::new().framed(framed.into_inner());
                        return Ok(framed);
                    } else {
                        warn!("server reject handshake");
                    }
                }
                _ => {
                    error!("Handshake resp not match");
                }
            },
            Some(Err(err)) => {
                error!("Handshake resp decode error: {}", err);
            }
            None => {
                error!("Handshake connection closed")
            }
        }
        bail!("Handshake failed")
    }

    async fn framed_loop(url: &str) -> Framed {
        let mut wait = Duration::from_secs(2);
        let max = Duration::from_secs(20);

        loop {
            match Self::framed(url).await {
                Ok(f) => break f,
                Err(err) => {
                    error!("Connection error: {}", err);
                    wait += Duration::from_secs(2);
                    wait = wait.min(max);
                }
            }
            time::sleep(wait).await;
        }
    }

    async fn framed(url: &str) -> Result<Framed> {
        info!("connecting server");
        let tcp = TcpStream::connect(url).await?;
        let framed = Self::handshake(tcp).await?;
        Ok(framed)
    }

    async fn next(&mut self) -> ManagerMsg {
        loop {
            let msg = self.conn.next().await;
            match msg {
                Some(Ok(msg)) => break msg,
                Some(Err(err)) => {
                    error!("Error: {}", err);
                }
                None => {
                    error!(%self.url, "Connection closed");
                }
            }

            self.conn = Self::framed_loop(&self.url).await;
        }
    }

    async fn send<O: Serialize>(&mut self, msg: WorkerMsg<O>) -> Result<()> {
        self.conn.send(msg).await?;
        Ok(())
    }
}
