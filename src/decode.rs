use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::analyze::{Analyzer, Rgb8};
use crate::error::StriskError;

#[derive(Clone, Copy)]
pub struct DecodeParams {
    pub sample_width: u32,
    pub scaled_h: u32,
    pub levels: usize,
}

pub fn scaled_dims(sample_width: u32, src_w: u32, src_h: u32) -> (u32, u32) {
    let mut h = ((src_h as f64) * (sample_width as f64) / (src_w as f64))
        .round()
        .max(2.0) as u32;
    if h % 2 == 1 {
        h += 1;
    }
    (sample_width, h)
}

/// Decode one time segment [start, end) through ffmpeg and collect dominant colours.
fn decode_segment(
    input: &Path,
    start: Option<f64>,
    dur: Option<f64>,
    p: &DecodeParams,
) -> Result<Vec<Rgb8>, StriskError> {
    let (sw, sh) = (p.sample_width, p.scaled_h);
    let frame_bytes = (sw * sh * 3) as usize;

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error"]);
    if let Some(s) = start {
        cmd.args(["-ss", &format!("{:.3}", s)]);
    }
    cmd.arg("-i").arg(input);
    if let Some(d) = dur {
        cmd.args(["-t", &format!("{:.3}", d)]);
    }
    cmd.args(["-vf", &format!("scale={}:{}:flags=fast_bilinear", sw, sh)]);
    cmd.args(["-pix_fmt", "rgb24", "-f", "rawvideo", "-"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|_| StriskError::FfmpegMissing)?;
    let mut stdout = child.stdout.take().unwrap();
    let mut colors = Vec::new();
    let mut buf = vec![0u8; frame_bytes];
    let mut analyzer = Analyzer::new(p.levels);
    loop {
        match stdout.read_exact(&mut buf) {
            Ok(()) => colors.push(analyzer.dominant(&buf)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(StriskError::Decode(format!("ffmpeg exited {}", status)));
    }
    Ok(colors)
}

/// Stream-decode the video and collect one dominant colour per frame.
/// `jobs` > 1 splits the video into parallel time segments decoded by
/// separate ffmpeg processes, roughly scaling with core count.
pub fn analyze_video(
    input: &Path,
    duration_secs: Option<f64>,
    jobs: usize,
    p: &DecodeParams,
) -> Result<Vec<Rgb8>, StriskError> {
    let jobs = jobs.max(1);
    if jobs == 1 || duration_secs.is_none_or(|d| d <= 0.0) {
        let colors = decode_segment(input, None, None, p)?;
        if colors.is_empty() {
            return Err(StriskError::ZeroFrames);
        }
        return Ok(colors);
    }
    let dur = duration_secs.unwrap();
    let seg = dur / jobs as f64;
    let mut handles = Vec::new();
    for i in 0..jobs {
        let start = i as f64 * seg;
        let seg_dur = if i + 1 == jobs { dur - start } else { seg };
        let input = input.to_path_buf();
        let p = *p;
        handles.push(std::thread::spawn(move || {
            decode_segment(&input, Some(start), Some(seg_dur), &p)
        }));
    }
    let mut colors = Vec::new();
    for h in handles {
        let mut seg = h
            .join()
            .map_err(|_| StriskError::Decode("worker panicked".into()))??;
        colors.append(&mut seg);
    }
    if colors.is_empty() {
        return Err(StriskError::ZeroFrames);
    }
    Ok(colors)
}
