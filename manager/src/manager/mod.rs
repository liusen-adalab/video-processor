pub mod parse;
pub mod transcode;

use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    sync::OnceLock,
};

use parking_lot::RwLock;
use protocol::{
    manager_msg::JobRaw,
    worker_msg::{JobEventWraper, Progress},
    JobType, TaskId,
};
use rand::{distributions::WeightedIndex, prelude::Distribution};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, warn};
use utils::{id_new_type, macros::codec::bincode};

use crate::controller::worker::ProjectMsg;

pub struct TaskRaw {
    task_type: TaskType,
    payload: Vec<u8>,
}

pub enum TaskType {
    Parse,
}

#[derive(Serialize, Deserialize)]
pub enum TaskStatus {
    Created,
    Running(Progress),
    Done(String),
    Failed(String),
}

pub enum TaskResult {
    Success { id: TaskId, output: Option<String> },
    Failure { id: TaskId, reason: String },
}

pub trait IntoTaskRaw {
    fn into_raw(&self) -> TaskRaw;
}

impl<T> IntoTaskRaw for T
where
    T: Task,
{
    fn into_raw(&self) -> TaskRaw {
        let p = bincode::serialize(&self).unwrap();
        TaskRaw {
            payload: p,
            task_type: Self::ty(),
        }
    }
}

pub trait TryFromTaskRaw: Sized {
    fn try_from(raw: &TaskRaw) -> Result<Self, anyhow::Error>;
}

impl<T> TryFromTaskRaw for T
where
    T: Task,
{
    fn try_from(raw: &TaskRaw) -> Result<Self, anyhow::Error> {
        Ok(bincode::deserialize(&raw.payload)?)
    }
}

pub trait Task: IntoTaskRaw + TryFromTaskRaw + Send + DeserializeOwned + Serialize {
    fn ty() -> TaskType;

    fn id(&self) -> TaskId;

    fn status(&self) -> &TaskStatus;

    fn result(&self) -> Option<TaskResult>;

    fn next_job(&mut self) -> Option<JobRaw>;

    fn have_next_job(&self) -> bool;
}

pub trait JobEventHandler<O>: Task {
    fn handle_event(&mut self, event: JobEventWraper<O>);
}

pub trait JobOutput: DeserializeOwned + Send + 'static {
    type Task: JobEventHandler<Self>;
}

trait TaskResultSubscriber {
    fn on_task_result(&mut self, result: TaskResult);
}

impl TaskResultSubscriber for () {
    fn on_task_result(&mut self, _: TaskResult) {}
}

pub struct ManagerInner<TR, WR> {
    task_repo: TR,
    worker_repo: WR,
    task_result_subscritber: Box<dyn TaskResultSubscriber + Send + Sync>,
    job_queue: HashMap<JobType, VecDeque<JobRaw>>,
}

pub trait TaskRepo {
    fn get<T>(&mut self, task_id: TaskId) -> Option<T>
    where
        T: Task;

    fn save<T>(&mut self, task: T)
    where
        T: Task;

    fn del(&mut self, task_id: TaskId);

    fn exist(&self, task_id: TaskId) -> bool;
}

pub trait WorkerRepo {
    fn rand_idle_worker(&mut self, ty: JobType) -> Option<&mut Worker>;

    fn save(&mut self, worker: Worker);
}

id_new_type!(WorkerId);

pub struct Worker {
    id: WorkerId,
    ty: JobType,
    weight: u64,
    sender: UnboundedSender<ProjectMsg>,
    power: WorkerPower,
}

pub struct WorkerPower {
    total: u32,
    used: u32,
}

impl Worker {
    pub fn new(sender: UnboundedSender<ProjectMsg>, ty: JobType) -> Self {
        Self {
            id: WorkerId::next_id(),
            weight: 1,
            sender,
            ty,
            power: WorkerPower { total: 10, used: 0 },
        }
    }

    fn new_job(&mut self, job: JobRaw) {
        if let Err(_) = self.sender.send(ProjectMsg::Job(job)) {
            error!(%self.id, "worker send job failed");
        }
    }

    fn has_available_power(&self) -> bool {
        self.power.used < self.power.total
    }
}

#[derive(Default)]
pub struct BinCodeTaskRepo {
    tasks: HashMap<TaskId, TaskRaw>,
}

impl TaskRepo for BinCodeTaskRepo {
    fn get<T>(&mut self, task_id: TaskId) -> Option<T>
    where
        T: Task,
    {
        if let Some(task) = self.tasks.get(&task_id) {
            match T::try_from(task) {
                Ok(t) => Some(t),
                Err(err) => {
                    error!("{}", err);
                    None
                }
            }
        } else {
            None
        }
    }

    fn save<T>(&mut self, task: T)
    where
        T: Task,
    {
        self.tasks.insert(task.id(), task.into_raw());
    }

    fn del(&mut self, task_id: TaskId) {
        self.tasks.remove(&task_id);
    }

    fn exist(&self, task_id: TaskId) -> bool {
        self.tasks.contains_key(&task_id)
    }
}

#[derive(Default)]
pub struct WeightWorkerRepo {
    workers: HashMap<JobType, Vec<Worker>>,
}

impl WorkerRepo for WeightWorkerRepo {
    fn rand_idle_worker(&mut self, ty: JobType) -> Option<&mut Worker> {
        let workers = self.workers.get_mut(&ty)?;
        let mut workers: Vec<_> = workers
            .into_iter()
            .filter(|w| w.has_available_power())
            .collect();
        let wieghts = workers.iter().map(|w| w.weight).collect::<Vec<_>>();
        let dist = WeightedIndex::new(wieghts).unwrap();
        Some(workers.remove(dist.sample(&mut rand::thread_rng())))
    }

    fn save(&mut self, worker: Worker) {
        self.workers.entry(worker.ty).or_default().push(worker);
    }
}

pub struct Manager<TR, WR> {
    inner: RwLock<ManagerInner<TR, WR>>,
}

pub fn global_manager() -> &'static Manager<BinCodeTaskRepo, WeightWorkerRepo> {
    static MANAGER: OnceLock<Manager<BinCodeTaskRepo, WeightWorkerRepo>> = OnceLock::new();
    MANAGER.get_or_init(|| {
        let inner = ManagerInner {
            task_repo: BinCodeTaskRepo::default(),
            worker_repo: WeightWorkerRepo::default(),
            task_result_subscritber: Box::new(()),
            job_queue: Default::default(),
        };
        Manager {
            inner: RwLock::new(inner),
        }
    })
}

impl<Tr, Wr> Manager<Tr, Wr>
where
    Tr: TaskRepo,
    Wr: WorkerRepo,
{
    pub fn job_event<E, T>(&self, event: JobEventWraper<E>)
    where
        E: JobOutput,
        T: JobEventHandler<E>,
    {
        let mut inner = self.inner.write();
        inner.job_event::<_, T>(event);
    }

    pub fn new_worker(&self, sender: UnboundedSender<ProjectMsg>, ty: JobType) {
        let worker = Worker::new(sender, ty);
        let mut inner = self.inner.write();
        inner.new_worker(worker);
    }
}

impl<TR, WR> ManagerInner<TR, WR>
where
    TR: TaskRepo,
    WR: WorkerRepo,
{
    pub fn new_worker(&mut self, worker: Worker) {
        self.worker_repo.save(worker);
    }

    pub fn new_task<T: Task + Debug>(&mut self, mut task: T) {
        if self.task_repo.exist(task.id()) {
            warn!(?task, "task already exist");
            return;
        }
        self.collect_jobs(&mut task);
        self.task_repo.save(task);
    }

    pub fn job_event<E, T>(&mut self, event: JobEventWraper<E>)
    where
        E: JobOutput,
        T: JobEventHandler<E>,
    {
        let Some(mut task)= self.task_repo.get::<T>(1) else {
            return;
        };

        task.handle_event(event);

        if let Some(result) = task.result() {
            self.task_result_subscritber.on_task_result(result);
            self.task_repo.del(task.id());
        } else {
            if task.have_next_job() {
                self.collect_jobs(&mut task);
                self.dispatch_job();
            }

            self.task_repo.save(task)
        }
    }

    fn collect_jobs<T: Task>(&mut self, task: &mut T) {
        while let Some(job) = task.next_job() {
            let queue = self.job_queue.entry(job.ty).or_default();
            queue.push_back(job.into());
        }
    }

    fn dispatch_job(&mut self) {
        for (ty, queue) in self.job_queue.iter_mut() {
            if queue.is_empty() {
                continue;
            }

            let Some(mut worker) = self.worker_repo.rand_idle_worker(*ty) else {
                continue;
            };

            while let Some(job) = queue.pop_front() {
                // ?dead
                worker.new_job(job);

                match self.worker_repo.rand_idle_worker(*ty) {
                    Some(w) => worker = w,
                    None => break,
                }
            }
        }
    }
}
