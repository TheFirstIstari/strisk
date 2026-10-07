# strisk

Render a video's per-frame dominant colours as a radial disk PNG.

Each frame of the input video contributes a wedge of a transparent-background ring,
coloured by that frame's predominant colour. Frames radiate clockwise from a blank
inner circle; a transparent notch at 12 o'clock marks the start/end boundary.

## Requirements

- Rust (stable)
- `ffmpeg` and `ffprobe` on `PATH`

## Usage

```sh
strisk <INPUT> [OPTIONS]

Options:
  -o, --output <PATH>       Output PNG path [default: <input_stem>.strisk.png]
  -r, --radius <PX>         Outer disk radius [default: from --output-res]
      --output-res <PX>    Output image side length [default: 8192]
      --inner-fraction <F>  Blank center radius as fraction of R [default: 0.25]
      --notch-degrees <DEG> Width of the start/end notch [default: 4.0]
      --levels <L>          Quantization levels per RGB channel [default: 8]
      --sample-width <W>    Decoded analysis width in px [default: 64]
      --jobs <N>             Parallel ffmpeg decode processes [default: 4]
```

Note: output side is capped at 16384px (~1GB RGBA buffer).

Example:

```sh
strisk movie.mp4 -o movie.png -r 1024
```

## Development

Using [mise](https://mise.jdx.dev) tasks:

```sh
mise run build      # cargo build
mise run release    # cargo build --release
mise run test       # cargo test
mise run clippy     # cargo clippy -- -D warnings
mise run fmt        # cargo fmt
mise run run -- movie.mp4 -r 1024   # build release + run the program
```

Or plain cargo: `cargo test`, `cargo build --release`.

See [SPEC.md](SPEC.md) for the full design specification.
