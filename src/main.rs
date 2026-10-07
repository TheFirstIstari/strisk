mod analyze;
mod cli;
mod decode;
mod error;
mod png_stream;
mod probe;
mod render;

use clap::Parser;

use cli::Cli;
use error::StriskError;

fn run() -> Result<(), StriskError> {
    let cli = Cli::parse();
    cli.validate().map_err(StriskError::InvalidOption)?;

    let probed = probe::probe(&cli.input)?;
    let scaled_h = decode::scaled_dims(cli.sample_width, probed.width, probed.height).1;
    let params = decode::DecodeParams {
        sample_width: cli.sample_width,
        scaled_h,
        levels: cli.levels,
    };
    let colors = decode::analyze_video(
        &cli.input,
        probed.duration_secs,
        cli.jobs,
        &params,
    )?;

    if let Some(expected) = probed.frame_count {
        if colors.len().abs_diff(expected) > cli.jobs {
            eprintln!(
                "note: decoded {} frames, probed {} (segment-boundary drift)",
                colors.len(),
                expected
            );
        }
    }
    let mut radius = cli.radius.unwrap_or_else(|| (cli.output_res.saturating_sub(2)) / 2);
    let max_r = (render::MAX_SIDE - 2) / 2;
    if radius > max_r {
        eprintln!(
            "note: radius {}px exceeds max canvas; clamped to {}px (frames will alias)",
            radius, max_r
        );
        radius = max_r;
    }
    let params = render::RenderParams {
        radius,
        inner_fraction: cli.inner_fraction,
        notch_degrees: cli.notch_degrees,
    };
    let side = (2 * radius + 2) as usize;
    let mut row_buf = vec![0u8; side * 4];
    png_stream::write_png_streaming(
        &cli.output_path(),
        side as u32,
        side as u32,
        |y, out| {
            row_buf.iter_mut().for_each(|b| *b = 0);
            render::render_row(&colors, &params, side, y, &mut row_buf);
            out.copy_from_slice(&row_buf);
        },
    )?;
    eprintln!("wrote {} ({} frames, radius {}px)", cli.output_path().display(), colors.len(), radius);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
