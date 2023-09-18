use tracing::info;
use worker::worker::WorkerCommonCfg;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    utils::logger::init(&utils::logger::Config {
        level: "debug".to_string(),
    })?;

    info!("running worker");
    run().await;

    std::future::pending().await
}

async fn run() {
    let cfg = Box::new(WorkerCommonCfg {
        worker_type: protocol::JobType::Parse,
        url: "127.0.0.1:8388".to_string(),
    });
    let cfg = Box::leak(cfg);

    worker::worker::start(cfg);
}
