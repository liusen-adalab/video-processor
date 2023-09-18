use anyhow::{bail, Context, Result};
use futures::{SinkExt, StreamExt};
use protocol::{
    handshake::{HandShakeProtocol, HandShakeResp},
    manager_msg::{JobRaw, ManagerCmd, ManagerMsg},
    parse::ParseOutput,
    worker_msg::WorkerMsg,
    JobType,
};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
};
use tracing::{error, info};
use utils::{
    codec, log_if_err,
    macros::codec::tokio_util::codec::{Decoder, Framed},
};

use crate::manager::{self, global_manager, JobOutput};

codec!(ServerCodec, encode: ManagerMsg, decode: WorkerMsg<O>);

pub enum ProjectMsg {
    Stop,
    Job(JobRaw),
}

codec!(HandShakeCodec, encode: HandShakeProtocol, decode: HandShakeProtocol);

pub async fn listener(bind: &str) -> Result<()> {
    let listener = TcpListener::bind(bind).await?;
    let (tcp, addr) = listener.accept().await?;
    info!(?addr, "new worker connected");

    tokio::spawn(async move {
        log_if_err!(init_connect(tcp).await);
    });

    todo!()
}

pub async fn init_connect(tcp: TcpStream) -> Result<()> {
    let mut framed = HandShakeCodec::new().framed(tcp);
    match framed.next().await {
        Some(Ok(msg)) => {
            let ty = handshake(msg, &mut framed).await?;
            spawn_router(framed.into_inner(), ty);
        }
        Some(err) => {
            err.context("decode handshake msg error")?;
        }
        None => {
            bail!("worker connection closed without handshake");
        }
    }
    Ok(())
}

async fn handshake(
    msg: HandShakeProtocol,
    framed: &mut Framed<TcpStream, HandShakeCodec>,
) -> Result<JobType> {
    match msg {
        HandShakeProtocol::Req(req) => {
            let info = req.system;
            info!(?info, "todo: worker system info");
            let resp = HandShakeProtocol::Resp(HandShakeResp {
                accepted: true,
                msg: None,
            });
            framed.send(resp).await?;

            Ok(req.worker_type)
        }
        HandShakeProtocol::Resp(_) => {
            bail!("worker didn't follow handshake protocol")
        }
    }
}

fn spawn_router(tcp: TcpStream, ty: JobType) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let manager = manager::global_manager();
    manager.new_worker(tx, ty);
    let framed = ServerCodec::new().framed(tcp);
    match ty {
        JobType::Parse => {
            let router = WorkerRouter::<ParseOutput> {
                mailbox: rx,
                framed,
            };
            router.spawn();
        }
    }
}

pub struct WorkerRouter<O> {
    mailbox: tokio::sync::mpsc::UnboundedReceiver<ProjectMsg>,
    framed: Framed<TcpStream, ServerCodec<O>>,
}

impl<O: Send + 'static> WorkerRouter<O>
where
    O: JobOutput,
{
    fn spawn(self) {
        tokio::spawn(async move {
            log_if_err!(self.serve().await);
        });
    }

    async fn serve(mut self) -> Result<()> {
        loop {
            select! {
                cmd = self.mailbox.recv() => {
                    match cmd {
                        Some(ProjectMsg::Stop) => {},
                        Some(ProjectMsg::Job(job)) => {
                            self.framed
                                .send(ManagerMsg {
                                    cmd: ManagerCmd::Job(job),
                                })
                                .await?;
                        }
                        None => {
                            info!("worker cmd chan closed");
                            break;
                        }
                    }
                }
                msg = self.framed.next() => {
                    match msg {
                        Some(Ok(msg)) => self.handle_msg(msg).await?,
                        Some(Err(err)) => {
                            error!("worker connection error: {}", err);
                            break;
                        },
                        None => {
                            info!("worker connection closed");
                            break;
                        },
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_msg(&mut self, msg: WorkerMsg<O>) -> Result<()> {
        match msg {
            WorkerMsg::JobEvent(event) => {
                let manger = global_manager();
                manger.job_event::<_, O::Task>(event);
            }
            WorkerMsg::SystemInfo(info) => {
                info!("worker system info: {:?}", info);
            }
        }
        Ok(())
    }
}
