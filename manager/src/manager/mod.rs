mod transcode;

use std::collections::{HashMap, VecDeque};

pub struct TaskRaw {}

pub enum TaskStatus {
    Running(Progress),
    Done,
    Failed(String),
}

#[allow(unused)]
pub struct Progress {
    total: u64,
    current: u64,
}

pub trait Task: TryFrom<TaskRaw> {
    type Job: Job<Task = Self>;

    fn id(&self) -> i64;

    fn status(&self) -> TaskStatus;

    fn result(&self) -> Option<TaskResult>;

    fn handle_event<J: JobEvent>(&mut self, event: &J);

    fn next_job(&mut self) -> Option<Self::Job>;

    fn have_next_job(&self) -> bool;
}

pub trait JobEvent {
    type Task: Task;
    type Job: Job<Task = Self::Task>;

    fn job_id(&self) -> i64;

    fn task_id(&self) -> i64;
}

pub trait Job: serde::Serialize + Into<JobRaw> {
    type Task: Task;

    fn worker_type(&self) -> WorkerType;
}

#[derive(Hash, Eq, PartialEq, Clone, Debug, Copy)]
pub enum WorkerType {
    Parse,
}

pub trait TaskRepo {
    fn get<T>(&mut self, task_id: i64) -> Option<T>
    where
        T: Task;

    fn save<T>(&mut self, task: T)
    where
        T: Task;

    fn del(&mut self, task_id: i64);

    fn exist(&self, task_id: i64) -> bool;
}

#[allow(unused)]
pub struct Worker {
    id: i64,
    type_: WorkerType,
    weight: u64,
}

impl Worker {
    fn new_job(&mut self, _job: JobRaw) {}
}

pub trait WorkerRepo {
    fn rand_idle_worker(&mut self, ty: WorkerType) -> Option<Worker>;
}

pub enum TaskResult {
    Success(i64),
    Failure(String),
}

trait TaskResultSubscriber {
    fn on_task_result(&mut self, result: TaskResult);
}

pub struct Manager<TR, WR> {
    task_repo: TR,
    worker_repo: WR,
    task_result_subscritber: Box<dyn TaskResultSubscriber>,
    job_queue: HashMap<WorkerType, VecDeque<JobRaw>>,
}

pub struct JobRaw {}

impl<TR, WR> Manager<TR, WR>
where
    TR: TaskRepo,
    WR: WorkerRepo,
{
    pub fn new_task<T: Task>(&mut self, mut task: T) {
        if self.task_repo.exist(task.id()) {
            return;
        }
        self.collect_jobs(&mut task);
        self.task_repo.save(task);
    }

    pub fn job_event<E: JobEvent>(&mut self, event: E) {
        let Some(mut task)= self.task_repo.get::<E::Task>(event.task_id()) else {
            return;
        };

        task.handle_event(&event);
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
            let queue = self.job_queue.entry(job.worker_type()).or_default();
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
