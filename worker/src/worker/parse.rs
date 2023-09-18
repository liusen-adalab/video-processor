use std::path::PathBuf;

use anyhow::bail;
use async_process::Command;
use protocol::parse::ParseOutput;
use serde::{Deserialize, Serialize};
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

use super::{Job, JobEventWraper, TaskId, Worker};

pub struct ParseWorker {}

#[derive(Debug, Deserialize)]
pub struct ParseJob {
    task_id: TaskId,
    path: PathBuf,
}

pub struct SimpleHandler {
    cancel_chan: oneshot::Sender<()>,
}

impl super::WorkerHandler for SimpleHandler {
    fn cancel(self) {
        let _ = self.cancel_chan.send(());
    }
}

impl Worker for ParseWorker {
    type Job = ParseJob;

    type Output = ParseOutput;

    type Handler = SimpleHandler;

    fn spawn(job: Self::Job, chan: UnboundedSender<JobEventWraper<Self::Output>>) -> Self::Handler {
        let (tx, cancel_signal) = oneshot::channel();

        tokio::spawn(async move {
            let res;
            select! {
                _ = cancel_signal => {
                    return;
                }
                r = parse(&job.path) => {
                    res = r
                }
            }

            let event = match res {
                Ok(res) => {
                    let output = res.map(|o| serde_json::to_string(&o).unwrap());
                    crate::worker::JobEvent::Ok(output.into())
                }
                Err(err) => crate::worker::JobEvent::Err(format!("{:?}", err)),
            };

            let event = JobEventWraper {
                task_id: job.task_id,
                event,
                job_id: 0,
                job_type: protocol::JobType::Parse,
            };
            let _ = chan.send(event);
        });

        SimpleHandler { cancel_chan: tx }
    }

    fn worker_type() -> super::JobType {
        todo!()
    }
}

impl Job for ParseJob {
    fn task_id(&self) -> super::TaskId {
        todo!()
    }
}

use std::path::Path;

use anyhow::{Context, Result};
use paste::paste;
use serde::Deserializer;
use tracing::debug;
use tracing::info;
use tracing::warn;

macro_rules! media_type {
    ($type_:ident) => {
        paste::paste! {
            #[derive(Debug, Clone)]
            pub struct [<$type_ Type>];

            fn [<de_ $type_:snake _type>]<'de, D>(deserializer: D) -> Result<[<$type_ Type>], D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let ss = <String as serde::Deserialize>::deserialize(deserializer)?;
                if ss.eq_ignore_ascii_case(stringify!($type_)) {
                    Ok([<$type_ Type>])
                } else {
                    Err(::serde::de::Error::custom(stringify!(not a $type_)))
                }
            }
        }
    };
}

media_type!(Video);
media_type!(Audio);
media_type!(General);

macro_rules! from_str {
    ($type_:ident) => {
        paste! {
        #[allow(unused)]
        fn [<$type_ _from_str>]<'de, D>(deserializer: D) -> Result<$type_, D::Error>
        where
            D: Deserializer<'de>,
        {
            let ss = String::deserialize(deserializer)?;
            ss.parse().map_err(::serde::de::Error::custom)
        }

        }
    };

    ($type_:ident, opt) => {
        paste! {
            fn [<$type_ _from_str_opt>]<'de, D>(deserializer: D) -> Result<Option<$type_>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let ss = Option::<String>::deserialize(deserializer)?;
                let res = ss
                    .map(|ss| ss.parse::<$type_>())
                    .transpose()
                    .map_err(::serde::de::Error::custom)?;
                Ok(res)
            }

        }
    };
}

from_str!(f64, opt);
from_str!(f64);
from_str!(u8, opt);
from_str!(u8);
from_str!(u32, opt);
from_str!(u32);
from_str!(u64, opt);

pub async fn parse(path: &Path) -> anyhow::Result<Option<MediaInfo>> {
    debug!(?path, "parsing video");
    let p = Command::new("mediainfo")
        .args(["--Output=JSON", &*path.to_string_lossy()])
        .spawn()?;
    let output = p.output().await?;
    if !output.status.success() {
        bail!("parse cmd failed");
    }

    let output = String::from_utf8(output.stdout)?;
    let mut output = MediaInfo::parse_cmd_output(output)?;
    let audio_id = get_default_audio_id(path)?;
    if let Some(output) = &mut output {
        output.ext.default_audio_track_id = audio_id;
    }
    Ok(output)
}

pub fn get_default_audio_id(path: &Path) -> anyhow::Result<Option<u32>> {
    // mediainfo "--Output=Audio;[%ID/String%, ][%Default/String%. ]" --Language="  Config_Text_ThousandsSeparator;" <input>
    let path = path.to_string_lossy();
    let args = vec!["--Output=Audio;[%ID/String%, ][%Default/String%. ]", &*path];
    let mut cmd = std::process::Command::new("mediainfo");
    cmd.args(args);
    let out = cmd.output()?;

    // 2, Yes. 3, No. 4, No.
    let out = String::from_utf8(out.stdout).context("audio id output is not utf-8 encoded")?;
    let out: Vec<_> = out.trim().split(".").filter(|s| !s.is_empty()).collect();
    if out.is_empty() {
        return Ok(None);
    }

    for audio in &out {
        if let [id, is_default] = audio.split(",").collect::<Vec<_>>()[..] {
            debug!(%id, %is_default, "parsing audio info");
            let id: u32 = id.parse().context("parse audio id")?;
            let is_fault = is_default.trim().eq_ignore_ascii_case("Yes");
            if is_fault {
                return Ok(Some(id));
            }
        } else {
            warn!(%path, ?out, "cannot extract mediainfo from video");
            return Ok(None);
        }
    }

    Ok(None)
}

impl MediaInfo {
    fn parse_cmd_output(cmd_output: String) -> Result<Option<Self>> {
        debug!("deserializing mediainfo");
        let output: serde_json::Value = serde_json::from_str(&cmd_output).unwrap();
        // 无论文件是否有效视频，都会输出这样的基本结构
        let output = output.as_object().unwrap();
        let media = output.get("media").unwrap().as_object().unwrap();
        let track = media.get("track").unwrap().as_array().unwrap();
        // general 总是列表中的第一个对象，且必定存在
        let general = track.get(0).unwrap();

        let general = match serde_json::from_value::<GeneralInfo>(general.clone()) {
            Ok(v) => v,
            Err(err) => {
                warn!(%cmd_output, ?err, "not a vaild file");
                return Ok(None);
            }
        };

        if general.VideoCount.is_none() || general.VideoCount == Some(0) {
            warn!("no video in file");
            return Ok(None);
        }

        let mut videos = vec![];
        let mut audios = vec![];
        let mut errs = vec![];

        for v in track.into_iter().skip(1) {
            match serde_json::from_value::<VideoInfo>(v.clone()) {
                Ok(vd) => {
                    videos.push(vd);
                    continue;
                }
                Err(err) => {
                    errs.push(err);
                }
            }
            match serde_json::from_value::<AudioInfo>(v.clone()) {
                Ok(vd) => {
                    audios.push(vd);
                }
                Err(err) => {
                    errs.push(err);
                }
            }
        }

        if videos.len() < 1 {
            warn!(%cmd_output, ?errs, "no video stream in file");
            return Ok(None);
        }
        if videos.len() > 1 {
            info!(?videos, "a video file with two video stream");
        }
        if audios.len() > 1 {
            info!(?audios, "a video file with two audio stream");
        }

        let default_audio = if audios.is_empty() {
            None
        } else {
            let mut default_audio = audios.remove(0);
            for audio in audios {
                if audio
                    .Default
                    .as_deref()
                    .unwrap_or_default()
                    .eq_ignore_ascii_case("yes")
                {
                    default_audio = audio;
                    break;
                }
            }
            Some(default_audio)
        };

        let info = MediaInfo {
            general,
            video: videos.remove(0),
            audio: default_audio,
            ext: MediaExtInfo {
                default_audio_track_id: None,
            },
        };
        Ok(Some(info))
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Media {
    media: Option<MediaInfo>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct MediaInfo {
    pub general: GeneralInfo,
    pub video: VideoInfo,
    pub audio: Option<AudioInfo>,
    pub ext: MediaExtInfo,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct MediaExtInfo {
    default_audio_track_id: Option<u32>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct GeneralInfo {
    #[serde(skip_serializing)]
    #[serde(rename = "@type")]
    #[serde(deserialize_with = "de_general_type")]
    _type: GeneralType,

    #[serde(default)]
    Format: Option<String>,

    #[serde(deserialize_with = "u8_from_str_opt")]
    #[serde(default)]
    VideoCount: Option<u8>,

    #[serde(deserialize_with = "u8_from_str_opt")]
    #[serde(default)]
    AudioCount: Option<u8>,

    #[serde(default)]
    DataSize: Option<String>,

    #[serde(deserialize_with = "f64_from_str_opt")]
    #[serde(default)]
    Duration: Option<f64>,

    #[serde(deserialize_with = "u32_from_str_opt")]
    #[serde(default)]
    pub OverallBitRate: Option<u32>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct VideoInfo {
    #[serde(skip_serializing)]
    #[serde(rename = "@type")]
    #[serde(deserialize_with = "de_video_type")]
    _type: VideoType,

    #[serde(default)]
    Format: Option<String>,
    #[serde(default)]
    Format_Level: Option<String>,
    #[serde(default)]
    Format_Profile: Option<String>,
    #[serde(default)]
    Format_Settings_CABAC: Option<String>,
    #[serde(default)]
    Format_Settings_GOP: Option<String>,
    #[serde(default)]
    Format_Settings_RefFrames: Option<String>,

    #[serde(default)]
    CodecID: Option<String>,

    #[serde(deserialize_with = "f64_from_str_opt")]
    #[serde(default)]
    pub Duration: Option<f64>,

    #[serde(deserialize_with = "u32_from_str_opt")]
    #[serde(default)]
    BitRate: Option<u32>,

    #[serde(deserialize_with = "u32_from_str_opt")]
    #[serde(default)]
    Width: Option<u32>,
    #[serde(deserialize_with = "u32_from_str_opt")]
    #[serde(default)]
    Height: Option<u32>,

    #[serde(default)]
    DisplayAspectRatio: Option<String>,
    #[serde(default)]
    FrameRate_Mode: Option<String>,

    #[serde(deserialize_with = "f64_from_str_opt")]
    #[serde(default)]
    FrameRate: Option<f64>,

    #[serde(deserialize_with = "u32_from_str_opt")]
    #[serde(default)]
    FrameCount: Option<u32>,

    #[serde(default)]
    ColorSpace: Option<String>,
    #[serde(default)]
    ChromaSubsampling: Option<String>,

    #[serde(deserialize_with = "u8_from_str_opt")]
    #[serde(default)]
    BitDepth: Option<u8>,

    #[serde(default)]
    ScanType: Option<String>,

    #[serde(deserialize_with = "u64_from_str_opt")]
    #[serde(default)]
    StreamSize: Option<u64>,

    #[serde(default)]
    colour_range: Option<String>,
    #[serde(default)]
    colour_primaries: Option<String>,

    #[serde(default)]
    transfer_characteristics: Option<String>,
    #[serde(default)]
    matrix_coefficients: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct AudioInfo {
    #[serde(skip_serializing)]
    #[serde(rename = "@type")]
    #[serde(deserialize_with = "de_audio_type")]
    _type: AudioType,

    #[serde(default)]
    Format: Option<String>,

    #[serde(deserialize_with = "f64_from_str_opt")]
    #[serde(default)]
    Duration: Option<f64>,

    #[serde(default)]
    BitRate_Mode: Option<String>,

    #[serde(deserialize_with = "u32_from_str_opt")]
    #[serde(default)]
    BitRate: Option<u32>,

    #[serde(deserialize_with = "u8_from_str_opt")]
    #[serde(default)]
    Channels: Option<u8>,

    ChannelLayout: Option<String>,

    #[serde(deserialize_with = "f64_from_str_opt")]
    #[serde(default)]
    FrameRate: Option<f64>,

    #[serde(default)]
    Compression_Mode: Option<String>,

    #[serde(default)]
    Default: Option<String>,

    #[serde(deserialize_with = "u64_from_str_opt")]
    #[serde(default)]
    StreamSize: Option<u64>,
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[tracing_test::traced_test]
    fn t_parse_output() {
        let mut output = std::fs::File::open("./parsed.json").unwrap();
        let mut out = String::new();
        output.read_to_string(&mut out).unwrap();

        assert!(dbg!(MediaInfo::parse_cmd_output(out)).is_ok());
    }

    #[test]
    fn t_get_audio_default_id() -> anyhow::Result<()> {
        let path = Path::new("./cats.mp4");
        dbg!(get_default_audio_id(path))?;
        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn valid() {
        let path = Path::new("./cats.mp4");

        let _ = dbg!(parse(path).await);
    }

    #[tokio::test]
    #[traced_test]
    async fn two_audio() {
        let path = Path::new("/home/sen/code/rust-ffmpeg/2024547333.mp4");
        let out_dir = Path::new("aa");
        let _ = std::fs::remove_dir_all(out_dir);
        std::fs::create_dir_all(out_dir).unwrap();

        let _ = dbg!(parse(path).await);
    }

    #[tokio::test]
    #[traced_test]
    async fn no_audio() {
        let path = Path::new("/home/sen/code/rust-ffmpeg/cats_no_audio.mp4");
        let out_dir = Path::new("aa");
        std::fs::remove_dir_all(out_dir).unwrap();
        std::fs::create_dir_all(out_dir).unwrap();

        let _ = dbg!(parse(path).await);
    }

    #[tokio::test]
    #[traced_test]
    async fn not_found() {
        let path = Path::new("./not-exist");
        let out_dir = Path::new("aa");
        std::fs::remove_dir_all(out_dir).unwrap();
        std::fs::create_dir_all(out_dir).unwrap();

        let _ = dbg!(parse(path).await);
    }

    #[tokio::test]
    #[traced_test]
    async fn regular_file() {
        let path = Path::new("./Cargo.toml");
        let out_dir = Path::new("aa");
        std::fs::remove_dir_all(out_dir).unwrap();
        std::fs::create_dir_all(out_dir).unwrap();

        let _ = dbg!(parse(path).await);
    }
}
