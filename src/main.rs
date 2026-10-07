mod analyze;
mod cli;
mod decode;
mod error;
mod png_out;
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
    let mut radius = cli.radius.unwrap_or_else(|| {
        render::auto_radius(colors.len(), cli.inner_fraction, cli.notch_degrees)
    });
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
    let rgba = render::render(&colors, &params);
    let side = 2 * radius + 2;
    png_out::write_png(&cli.output_path(), &rgba, side)?;
    eprintln!("wrote {} ({} frames, radius {}px)", cli.output_path().display(), colors.len(), radius);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
