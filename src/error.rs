use thiserror::Error;

#[derive(Debug, Error)]
pub enum StriskError {
    #[error("ffprobe not found on PATH; please install ffmpeg")]
    FfprobeMissing,
    #[error("ffmpeg not found on PATH; please install ffmpeg")]
    FfmpegMissing,
    #[error("probe failed: {0}")]
    Probe(String),
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("decode produced zero frames")]
    ZeroFrames,
    #[error("invalid option: {0}")]
    InvalidOption(String),
    #[error("png encode/write failed: {0}")]
    Png(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
