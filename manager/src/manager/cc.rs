use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::Arc,
};

use parking_lot::RwLock;
use tracing::warn;
use utils::id_new_type;

pub struct Job {
    task_id: TaskId,
}

pub struct JobEvent {
    pub job: Arc<Job>,
    pub output: JobOutput,
}

pub enum JobOutput {
    Parse(ParseOutput),
    Thumbnail,
}

pub struct ParseOutput {}

pub struct JobTracker {
    job: Arc<Job>,
    progress: Progress,
}

type JobTrackerCell = Arc<RwLock<JobTracker>>;

pub struct Progress {
    pub total: usize,
    pub current: usize,
}

id_new_type!(TaskId);
id_new_type!(WorkerId);

struct ManagerInner {
    tasks: HashMap<TaskId, Task>,
    workers: HashMap<WorkerId, Worker>,
    pending_jobs: HashMap<WorkerType, VecDeque<Job>>,
    working_jobs: HashMap<JobId, JobTrackerCell>,
}

pub struct TranscodeTaskTracker {
    task: Arc<TranscodeTask>,
    progress: Progress,
    step_record: TranscodeSteps,
}

pub struct TrancodeJobTracker {
    job: Arc<TrancodeJob>,
    progress: Progress,
}

pub enum TrancodeJob {
    Demux,
    Split,
    H264,
    Audio,
    Zcode,
    Mux,
}

pub enum TrancodeJobOutput {
    Split(SplitOutput),
}

pub struct SplitJobTracker {
    job: Arc<SplitJob>,
    progress: Progress,
}

pub struct SplitJob {
    task_id: TaskId,
    src: PathBuf,
    dst: PathBuf,
}

impl TranscodeTaskTracker {
    fn job_done(&mut self, job: JobId) -> Option<Vec<Job>> {
        todo!()
    }
}

pub struct SigleJobTaskTracker {
    task: Arc<Task>,
    progress: Progress,
}

pub struct TaskTracker {
    task: Arc<Task>,
    progress: Progress,
    step_record: TranscodeSteps,
}

pub struct Task {
    id: TaskId,
    ty: TaskType,
}

pub enum TaskType {
    Parse(ParseTaskParams),
    Transcode(TranscodeTask),
}

pub struct ParseTaskParams {
    path: PathBuf,
}

pub struct TranscodeTask {
    params: TranscodeTaskParams,
    step_record: TranscodeSteps,
    done_slice: HashSet<usize>,
    total_slice: usize,
}

use bitflags::bitflags;
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

pub struct Worker {
    id: WorkerId,
    ty: WorkerType,
    power: WorkerPower,
    mailbox: tokio::sync::mpsc::UnboundedSender<Job>,
    jobs: HashMap<JobId, JobTrackerCell>,
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

id_new_type!(JobId);

pub struct SplitOutput {}

impl ManagerInner {
    pub fn job_failed(&mut self, worker_id: WorkerId, job: JobId, reason: String) {
        todo!()
    }

    pub fn parse_done(&mut self, worker_id: WorkerId, job_id: JobId, output: ParseOutput) {
        let Some(worker) = self.workers.get_mut(&worker_id) else {
            warn!(%worker_id, %job_id,  "worker not found");
            return;
        };
        let Some(job) = worker.job_done(job_id) else {
            return;
        };
        let Some(task) = self.tasks.get_mut(&job.read().job.task_id) else {
            return
        };

        todo!()
    }

    pub fn split_done(&mut self, worker_id: WorkerId, job_id: JobId, output: SplitOutput) {
        let Some(worker) = self.workers.get_mut(&worker_id) else {
            warn!(%worker_id, %job_id,  "worker not found");
            return;
        };
        let Some(job) = worker.job_done(job_id) else {
            return;
        };
        let Some(task) = self.tasks.get_mut(&job.read().job.task_id) else {
            return
        };

        todo!()
    }

    pub fn zcode_done(&mut self, worker_id: WorkerId, job: JobId) {
        todo!()
    }

    pub fn zcode_progress(&mut self, worker_id: WorkerId, job: JobId, progress: Progress) {
        todo!()
    }
}

impl Worker {
    fn job_done(&mut self, job_id: JobId) -> Option<JobTrackerCell> {
        self.jobs.remove(&job_id)
    }
}

impl TaskTracker {
    fn split_done(&mut self, output: SplitOutput) -> Option<Vec<Job>> {
        todo!()
    }
}
