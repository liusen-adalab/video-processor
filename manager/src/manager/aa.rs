use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Arc,
};

use bitflags::bitflags;
use parking_lot::RwLock;
use tracing::{error, info, warn};
use utils::id_new_type;

id_new_type!(TaskId);
id_new_type!(WorkerId);
id_new_type!(JobId);

#[derive(Clone)]
pub struct Manager {
    inner: Arc<RwLock<ManagerInner>>,
}

struct ManagerInner {
    task_res_subscriber: tokio::sync::mpsc::UnboundedSender<TaskEvent>,
    tasks: HashMap<TaskId, Task>,
    workers: HashMap<WorkerId, Worker>,
    pending_jobs: HashMap<WorkerType, VecDeque<Job>>,
}

type JobCell = Arc<RwLock<Job>>;

pub struct Task {
    id: TaskId,
    progress: Progress,
    ty: TaskType,
    working_jobs: HashMap<JobId, JobCell>,
}

pub struct TranscodeTask {
    params: Arc<TranscodeTaskParams>,
    step_record: TranscodeSteps,
    done_slice: HashSet<usize>,
    total_slice: usize,
}

pub struct WorkerJobEvent {
    job_id: JobId,
    progress: Progress,
    output: JobResult,
}

pub enum JobResult {
    Done(Option<JobOutput>),
    Failed(String),
    Progressing(Progress),
}

pub enum JobOutput {
    Parse(ParseOutput),
    Split(SplitOutput),
}

pub struct SplitOutput {
    pub total_slice: usize,
}

pub struct ParseOutput {}

impl TranscodeTask {
    fn job_done2(
        &mut self,
        main_task_id: TaskId,
        job: &Job,
        event: WorkerJobEvent,
    ) -> Option<Vec<Job>> {
        debug_assert_eq!(job.task_id, main_task_id);
        debug_assert_eq!(job.id, event.job_id);

        match job.ty {
            JobType::Zcode(_) => {
                // self.done_slice.insert(slice_index);
                // // zcode done
                // if self.done_slice.len() == self.total_slice {
                //     self.step_record.insert(TranscodeSteps::ZcodeDone);
                // } else {
                //     return None;
                // }
            }
            JobType::Audio => {
                self.step_record.insert(TranscodeSteps::AudioDone);
            }
            JobType::H264 => {
                self.step_record.insert(TranscodeSteps::H264Done);
            }
            JobType::Demux => {
                if self.step_record.contains(TranscodeSteps::Demuxed) {
                    return None;
                }
                self.step_record.insert(TranscodeSteps::Demuxed);
                let next_jobs = vec![
                    Job::new(main_task_id, job.path.clone(), JobType::Audio),
                    Job::new(main_task_id, job.path.clone(), JobType::H264),
                ];
                return Some(next_jobs);
            }
            JobType::Split => {
                if let JobResult::Done(Some(JobOutput::Split(SplitOutput { total_slice }))) =
                    event.output
                {
                    self.total_slice = total_slice;
                } else {
                    error!("split job should have output");
                    return None;
                }

                if self.step_record.contains(TranscodeSteps::Splited) {
                    return None;
                }

                self.step_record.insert(TranscodeSteps::Splited);

                let mut next_jobs = vec![];
                for i in 0..self.total_slice {
                    let path = self.params.work_dir.join(format!("part{}.m4v", i));
                    next_jobs.push(Job::new(
                        main_task_id,
                        path,
                        JobType::Zcode(self.params.zcode.clone()),
                    ));
                }

                return Some(next_jobs);
            }
            JobType::Mux => todo!(),
            JobType::Parse => unreachable!(),
            JobType::Thumbnail(_) => unreachable!(),
        }
        todo!()
    }

    fn job_done(&mut self, main_task_id: TaskId, job: Job) -> Option<Vec<Job>> {
        todo!()
    }
}

pub enum TaskType {
    Parse(ParseTaskParams),
    Thumbnail(ThumbnailTaskParams),
    Transcode(TranscodeTask),
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TranscodeSteps: i16 {
        const Origin         = 0;
        const Demuxed        = 1 << 1;
        const AudioDone      = 1 << 2;
        const H264Done       = 1 << 3;
        const Splited        = 1 << 4;
        const ZcodeDone      = 1 << 5;
        const Muxed          = 1 << 6;
    }
}

pub struct ParseTaskParams {
    path: PathBuf,
}

pub struct ThumbnailTaskParams {
    path: PathBuf,
    dst_dir: PathBuf,
}

pub struct TranscodeTaskParams {
    work_dir: PathBuf,
    audio: AudioParameters,
    zcode: Arc<ZcodeParams>,
    mux: ContainerFormat,
}

use transcode_params::*;
mod transcode_params {
    #[derive(Clone)]
    pub struct ZcodeParams {
        pub format: VideoFormat,
        pub resolution: Option<Resolution>,
        pub ray_tracing: Option<RayTracing>,
        pub quality: OutputQuality,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VideoFormat {
        Av1,
        H264,
        H265,
    }

    #[derive(Clone)]
    pub enum Resolution {
        _144P,
        _240P,
        _360P,
        _480P,
        _720P,
        _1080P,
        _1440P,
        _4K,
    }

    #[derive(Clone)]
    pub enum RayTracing {
        Cg = 0,
        TvSeries = 8,
        Ordinary = 14,
        Lagacy = 25,
    }

    #[derive(Clone)]
    pub enum OutputQuality {
        Base = 5,
        High = 4,
        Top = 3,
    }

    // audio
    pub struct AudioParameters {
        pub format: AudioFormat,
        pub resample: AudioResampleRate,
        pub bitrate: AudioBitRate,
        pub track: AudioTrack,
    }

    pub enum AudioFormat {
        AAC,
        OPUS,
    }

    pub enum AudioResampleRate {
        _22050 = 22050,
        _44100 = 44100,
        _48000 = 48000,
    }

    pub enum AudioBitRate {
        _16 = 16,
        _32 = 32,
        _64 = 64,
        _128 = 128,
        _256 = 256,
        _320 = 320,
        _384 = 384,
        _640 = 640,
    }

    pub enum AudioTrack {
        _1 = 1,
        _2 = 2,
        _51 = 6,
        _71 = 8,
    }

    // mux
    pub enum ContainerFormat {
        Mp4,
        Webm,
        Mkv,
    }
}

#[derive(Clone)]
pub struct Progress {
    pub total: usize,
    pub current: usize,
}

pub struct Worker {
    id: WorkerId,
    ty: WorkerType,
    power: WorkerPower,
    mailbox: tokio::sync::mpsc::UnboundedSender<Job>,
    jobs: HashMap<JobId, JobCell>,
}

struct WorkerPower {
    total: usize,
    current: usize,
}

impl WorkerPower {
    fn available(&self) -> bool {
        self.current < self.total
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkerType {
    Parse,
    Thumbnail,

    Demux,
    Split,
    H264,
    Audio,
    Zcode,
    Mux,
}

#[derive(Clone)]
pub struct Job {
    id: JobId,
    task_id: TaskId,
    progress: Progress,
    path: PathBuf,
    ty: JobType,
}

impl Job {
    fn new(task_id: TaskId, path: PathBuf, ty: JobType) -> Self {
        Self {
            id: JobId::next_id(),
            task_id,
            progress: Progress {
                total: 1,
                current: 0,
            },
            path,
            ty,
        }
    }

    fn empty() -> Self {
        todo!()
    }

    fn job_record(&self) -> TranscodeSteps {
        match self.ty {
            JobType::Parse => TranscodeSteps::Origin,
            JobType::Thumbnail(_) => TranscodeSteps::Origin,
            JobType::Demux => TranscodeSteps::Demuxed,
            JobType::Split => TranscodeSteps::Splited,
            JobType::H264 => TranscodeSteps::H264Done,
            JobType::Audio => TranscodeSteps::AudioDone,
            JobType::Zcode { .. } => TranscodeSteps::Origin,
            JobType::Mux => TranscodeSteps::Muxed,
        }
    }
}

#[derive(Clone)]
pub enum JobType {
    Parse,
    Thumbnail(ThumbnailParams),

    Demux,
    Split,
    H264,
    Audio,
    Zcode(Arc<ZcodeParams>),
    Mux,
}

impl JobType {
    fn to_worker_type(&self) -> WorkerType {
        match self {
            JobType::Parse => WorkerType::Parse,
            JobType::Thumbnail(_) => WorkerType::Thumbnail,
            JobType::Demux => WorkerType::Demux,
            JobType::Split => WorkerType::Split,
            JobType::H264 => WorkerType::H264,
            JobType::Audio => WorkerType::Audio,
            JobType::Zcode { .. } => WorkerType::Zcode,
            JobType::Mux => WorkerType::Mux,
        }
    }
}

#[derive(Clone)]
pub struct ThumbnailParams {
    dst_dir: PathBuf,
}

pub struct TaskEvent {
    task_id: TaskId,
    event: TaskInnerEvent,
}

pub enum TaskInnerEvent {
    TaskDone,
    TaskFailed,
}

impl Manager {
    pub fn job_done(&self, worker_id: WorkerId, job_id: JobId) {
        let mut inner = self.inner.write();
        inner.job_done(worker_id, job_id);
    }

    pub fn job_failed(&self, worker_id: WorkerId, job_id: JobId) {
        todo!()
    }

    pub fn new_task(&self, task: Task) {
        if self.inner.read().tasks.get(&task.id).is_some() {
            warn!(%task.id, "task already exists");
            return;
        }

        let mut inner = self.inner.write();
        inner.new_task(task);
    }
}

impl ManagerInner {
    fn job_done(&mut self, worker_id: WorkerId, job_id: JobId) {
        let Some(worker) = self.workers.get_mut(&worker_id) else {
            warn!(%worker_id, %job_id,  "worker not found");
            return;
        };
        let Some(job) = worker.job_done(job_id) else {
            return;
        };
        let Some(task) = self.tasks.get_mut(&job.read().task_id) else {
            return
        };

        if let Some(next_jobs) = task.job_done(job_id) {
            for job in next_jobs {
                let pending_jobs = self
                    .pending_jobs
                    .entry(job.ty.to_worker_type())
                    .or_default();
                pending_jobs.push_back(job);
            }
            self.dispatch();
        } else {
            if task.is_done() {
                let id = task.id;
                self.tasks.remove(&id);
                self.send_task_done(id);
            }
        }
    }

    fn job_failed(&mut self, worker_id: WorkerId, job_id: JobId) {
        let Some(worker) = self.workers.get_mut(&worker_id) else {
            warn!(%worker_id, %job_id,  "worker not found");
            return;
        };
        let Some(job) = worker.job_done(job_id) else {
            return;
        };
        let Some(task) = self.tasks.get_mut(&job.read().task_id) else {
            return
        };

        if let Some(next_jobs) = task.job_done(job_id) {
            for job in next_jobs {
                let pending_jobs = self
                    .pending_jobs
                    .entry(job.ty.to_worker_type())
                    .or_default();
                pending_jobs.push_back(job);
            }
            self.dispatch();
        } else {
            if task.is_done() {
                let id = task.id;
                self.tasks.remove(&id);
                self.send_task_done(id);
            }
        }
    }

    fn find_available_worker(&mut self, ty: WorkerType) -> Option<&mut Worker> {
        let worker = self.workers.iter_mut().find(|(_, worker)| worker.ty == ty);
        worker.map(|(id, w)| w)
    }

    fn dispatch(&mut self) {
        let mut avail_workers: Vec<_> = self
            .workers
            .values_mut()
            .into_iter()
            .filter(|w| w.has_available_power())
            .collect();
        for worker in avail_workers.iter_mut() {
            let pending_jobs = self.pending_jobs.entry(worker.ty).or_default();
            while worker.has_available_power() {
                if let Some(job) = pending_jobs.pop_front() {
                    worker.add_job(job);
                } else {
                    break;
                }
            }
        }
    }

    fn new_task(&mut self, task: Task) {
        if self.tasks.get(&task.id).is_some() {
            warn!(%task.id, "task already exists");
            return;
        }
        let job = task.init_job();
        self.add_pending_job(job);

        self.tasks.insert(task.id, task);
        self.dispatch();
    }

    fn add_pending_job(&mut self, job: Job) {
        let pending_jobs = self
            .pending_jobs
            .entry(job.ty.to_worker_type())
            .or_default();
        pending_jobs.push_back(job);
    }

    fn send_task_done(&self, task_id: TaskId) {
        self.send_task_event(task_id, TaskInnerEvent::TaskDone)
    }

    fn send_task_event(&self, task_id: TaskId, event: TaskInnerEvent) {
        self.task_res_subscriber
            .send(TaskEvent { task_id, event })
            .unwrap();
    }
}

impl Worker {
    fn job_done(&mut self, job_id: JobId) -> Option<JobCell> {
        let job = self.jobs.remove(&job_id);
        match self.jobs.remove(&job_id) {
            None => {
                warn!(%job_id, "job not found");
                None
            }
            job => job,
        }
    }

    fn add_job(&mut self, job: Job) {
        debug_assert!(self.power.available());

        self.mailbox.send(job.clone()).unwrap();

        self.power.current += 1;
        self.jobs.insert(job.id, Arc::new(RwLock::new(job)));
    }

    fn has_available_power(&self) -> bool {
        self.power.available()
    }
}

impl Task {
    fn init_job(&self) -> Job {
        todo!()
    }

    fn job_done(&mut self, job_id: JobId) -> Option<Vec<Job>> {
        if let Some(job) = self.working_jobs.remove(&job_id) {
            let mut job = job.write();
            let added = job.progress.set_done();
            self.progress.current += added;

            match &mut self.ty {
                TaskType::Parse(_) => {}
                TaskType::Thumbnail(_) => {}
                TaskType::Transcode(TranscodeTask {
                    params,
                    step_record,
                    done_slice,
                    total_slice,
                }) => {
                    let record = job.job_record();
                    step_record.insert(record);
                    debug_assert!(step_record.contains(TranscodeSteps::Demuxed));

                    if *step_record == TranscodeSteps::Demuxed {
                        let mut next_jobs = vec![];
                        next_jobs.push(Job::new(self.id, job.path.clone(), JobType::Audio));
                        next_jobs.push(Job::new(self.id, job.path.clone(), JobType::H264));
                        return Some(next_jobs);
                    }

                    // if let JobType::Zcode { slice_index } = job.ty {
                    //     done_slice.insert(slice_index);
                    //     // zcode done
                    //     if done_slice.len() == *total_slice {
                    //         step_record.insert(TranscodeSteps::ZcodeDone);
                    //     } else {
                    //         return None;
                    //     }
                    // }

                    if step_record.contains(TranscodeSteps::AudioDone | TranscodeSteps::ZcodeDone) {
                        // mux
                        return Some(vec![Job::new(self.id, job.path.clone(), JobType::Mux)]);
                    }

                    let mut next_jobs = vec![];
                    if !step_record.contains(TranscodeSteps::Splited) {
                        next_jobs.push(Job::new(self.id, job.path.clone(), JobType::Split));
                    }

                    todo!("create next step jobs")
                }
            }
        }
        todo!()
    }

    fn job_failed(&mut self, job_id: JobId) -> Option<Vec<Job>> {
        todo!()
    }

    fn is_done(&self) -> bool {
        self.progress.is_done()
    }
}

impl Progress {
    fn set_done(&mut self) -> usize {
        let change = self.total - self.current;
        self.current = self.total;
        change
    }

    fn is_done(&self) -> bool {
        debug_assert!(self.current <= self.total);
        self.total == self.current
    }
}
