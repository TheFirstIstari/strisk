use serde::Deserialize;
use std::path::Path;
use std::process::Command;

use crate::error::StriskError;

#[derive(Debug, Deserialize)]
struct ProbeJson {
    streams: Vec<ProbeStream>,
}

#[derive(Debug, Deserialize)]
pub struct ProbeStream {
    #[serde(default)]
    pub nb_frames: Option<String>,
    #[serde(default)]
    pub nb_read_packets: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub avg_frame_rate: Option<String>,
}

#[derive(Debug)]
pub struct Probe {
    pub duration_secs: Option<f64>,
    pub frame_count: Option<usize>,
    pub width: u32,
    pub height: u32,
}

pub fn probe(input: &Path) -> Result<Probe, StriskError> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=nb_frames,nb_read_packets,width,height,duration,avg_frame_rate",
            "-of",
            "json",
        ])
        .arg(input)
        .output()
        .map_err(|_| StriskError::FfprobeMissing)?;
    if !out.status.success() {
        return Err(StriskError::Probe(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    let json: ProbeJson =
        serde_json::from_slice(&out.stdout).map_err(|e| StriskError::Probe(e.to_string()))?;
    let s = json
        .streams
        .into_iter()
        .next()
        .ok_or_else(|| StriskError::Probe("no video stream".into()))?;
    let width = s
        .width
        .ok_or_else(|| StriskError::Probe("missing width".into()))?;
    let height = s
        .height
        .ok_or_else(|| StriskError::Probe("missing height".into()))?;
    let frame_count = s
        .nb_frames
        .as_deref()
        .and_then(|v| v.parse().ok())
        .or_else(|| s.nb_read_packets.as_deref().and_then(|v| v.parse().ok()))
        .or_else(|| {
            let d: f64 = s.duration.as_deref()?.parse().ok()?;
            let fps = parse_rate(s.avg_frame_rate.as_deref()?)?;
            Some((d * fps).round() as usize)
        });
    let duration_secs = s.duration.as_deref().and_then(|d| d.parse().ok());
    Ok(Probe {
        frame_count,
        width,
        height,
        duration_secs,
    })
}

fn parse_rate(s: &str) -> Option<f64> {
    let (a, b) = s.split_once('/').unwrap_or((s, "1"));
    let a: f64 = a.parse().ok()?;
    let b: f64 = b.parse().ok()?;
    if b == 0.0 {
        None
    } else {
        Some(a / b)
    }
}
