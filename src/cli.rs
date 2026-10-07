use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "strisk", version, about = "Render a video's per-frame dominant colours as a radial disk PNG")]
pub struct Cli {
    /// Path to the input video file
    pub input: PathBuf,

    /// Output PNG path [default: <input_stem>.strisk.png]
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Outer disk radius in px [default: auto]
    #[arg(short, long)]
    pub radius: Option<u32>,

    /// Blank center radius as fraction of R
    #[arg(long, default_value_t = 0.25)]
    pub inner_fraction: f64,

    /// Width of the start/end notch in degrees
    #[arg(long, default_value_t = 4.0)]
    pub notch_degrees: f64,

    /// Quantization levels per RGB channel
    #[arg(long, default_value_t = 8)]
    pub levels: usize,

    /// Decoded analysis width in px
    #[arg(long, default_value_t = 64)]
    pub sample_width: u32,

    /// Parallel ffmpeg decode processes over time segments
    #[arg(long, default_value_t = 4)]
    pub jobs: usize,
}

impl Cli {
    pub fn validate(&self) -> Result<(), String> {
        if self.levels < 2 { return Err("levels must be >= 2".into()); }
        if !(self.inner_fraction > 0.0 && self.inner_fraction < 1.0) {
            return Err("inner_fraction must be in (0, 1)".into());
        }
        if self.notch_degrees < 0.0 || self.notch_degrees >= 60.0 {
            return Err("notch_degrees must be in [0, 60)".into());
        }
        if self.sample_width < 8 { return Err("sample_width must be >= 8".into()); }
        if let Some(r) = self.radius {
            if r < 16 { return Err("radius must be >= 16".into()); }
        }
        Ok(())
    }

    pub fn output_path(&self) -> PathBuf {
        self.output.clone().unwrap_or_else(|| {
            let stem = self.input.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
            self.input.with_file_name(format!("{}.strisk.png", stem))
        })
    }
}
