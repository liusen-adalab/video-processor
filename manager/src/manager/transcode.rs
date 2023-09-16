pub mod params {
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
