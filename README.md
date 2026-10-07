# strisk

[![CI](https://github.com/TheFirstIstari/strisk/actions/workflows/ci.yml/badge.svg)](https://github.com/TheFirstIstari/strisk/actions)

Render a video's per-frame dominant colours as a radial disk PNG.

Each frame of the input video contributes a wedge of a transparent-background ring,
coloured by that frame's predominant colour. Frames radiate clockwise from a blank
inner circle; a transparent notch at 12 o'clock marks the start/end boundary of the
video.

## Features

- 🎬 Any input format ffmpeg can read
- 🎨 Per-frame dominant colour via quantization-mode histogram (configurable levels)
- 🍩 Blank centre ring, configurable inner radius fraction
- 🕳️ Configurable notch marking the start/end boundary
- ⚡ Parallel multi-process ffmpeg decode (`--jobs`)
- 📉 Streaming, lossless PNG encoder — constant low memory, adaptive row filters
- 📐 Fixed output resolution target (default 8k), `-r` radius override

## Requirements

- Rust (stable)
- `ffmpeg` and `ffprobe` on `PATH`

## Installation

### Homebrew (macOS/Linux)

```sh
brew tap TheFirstIstari/strisk
brew install strisk
```

### From source

```sh
git clone https://github.com/TheFirstIstari/strisk.git
cd strisk
cargo build --release
# binary at target/release/strisk
```

## Usage

```sh
strisk <INPUT> [OPTIONS]

Options:
  -o, --output <PATH>          Output PNG path [default: <input_stem>.strisk.png]
  -r, --radius <PX>            Outer disk radius [default: from --output-res]
      --output-res <PX>        Output image side length [default: 8192]
      --inner-fraction <F>     Blank center radius as fraction of R [default: 0.25]
      --notch-degrees <DEG>    Width of the start/end notch [default: 4]
      --levels <L>             Quantization levels per RGB channel [default: 8]
      --sample-width <W>       Decoded analysis width in px [default: 64]
      --jobs <N>               Parallel ffmpeg decode processes [default: 4]
```

Examples:

```sh
strisk movie.mp4                              # 8k PNG next to the input
strisk movie.mp4 --output-res 16384 -o big.png
strisk movie.mkv --jobs 8 --notch-degrees 2
```

Memory: the encoder streams rows through a fixed row filter + zlib window —
the full canvas is never in memory (peak usage ≈ row buffer + 8MB IDAT slab).

## Development

Using [mise](https://mise.jdx.dev) tasks (`mise run build`, `mise run release`,
`mise run test`, `mise run clippy`, `mise run fmt`), or plain cargo:

```sh
cargo test          # unit + integration tests
cargo build --release
```

See [SPEC.md](SPEC.md) for the full design specification.

## License

[GPL-3.0-or-later](LICENSE)
