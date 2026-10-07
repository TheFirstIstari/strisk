# strisk — Project Specification

## 1. Overview

`strisk` is a Rust CLI program that analyzes a video/movie file and produces a single PNG image: a transparent-background disk (ring) where each frame of the video contributes a radial wedge colored by that frame's predominant colour. Frames radiate outward from a blank inner circle, progressing chronologically around the disk, with a transparent notch marking the boundary between the end and the start of the video.

## 2. Visual Output Definition

- Canvas: square of side `2 * R + 2` pixels, RGBA8.
- Outer disk radius: `R` pixels (user-settable, default auto-computed — see §4).
- Inner blank circle: radius `r_in = inner_fraction * R` (default inner_fraction = 0.25). Fully transparent.
- Retained ring: annulus `r_in < r <= R`.
- Notch: a wedge of configurable angular width `notch_degrees` (default 4.0°), centered on 12 o'clock (angle 0 = up, increasing clockwise), fully transparent. It separates the last frame (end of movie) from the first frame (00:00).
- Frame wedges: the remaining `360° - notch_degrees` of arc is divided equally among `N` frames. Frame `i` (0-indexed, chronological) occupies angles `[notch/2 + i * Δ, notch/2 + (i+1) * Δ)` where `Δ = (360° - notch_degrees) / N`.
- Pixel coverage: each pixel whose center falls in the retained ring is coloured by the frame whose wedge contains its angle. Pixels in the notch, outside the disk, or inside the inner circle have alpha = 0. Everything inside the disk/ring is fully opaque (alpha = 255) except the notch wedge (alpha 0).
- No anti-aliasing in v1 (hard edges); optional supersampling is a future enhancement.

## 3. Predominant Colour (per frame)

- Each decoded frame is spatially downscaled by ffmpeg (see §5) to a small size so analysis is cheap.
- Quantize each pixel's RGB to `L` levels per channel (default `L = 8`, configurable via `--levels`), producing an `L^3`-entry histogram (fixed-size `u32` array, e.g. 8^3 = 512 entries — no hash map).
- Winning bucket = bucket with the most pixels (ties: lowest bucket index; deterministic).
- Representative colour = arithmetic mean of the *original* RGB values of all pixels in the winning bucket (rounded to u8). This is stored as that frame's wedge colour.

## 4. Radius Default (auto)

When `--radius` is not given, choose the smallest `R` such that the arc length of every frame wedge is at least 1 pixel at the inner edge of the ring:

```
Δ_rad = (2π - notch_rad) / N
R >= 1 / (Δ_rad * inner_fraction)
```

i.e. `R = ceil(1 / (Δ_rad * inner_fraction))`, with a floor of 16 px. If `--radius` is provided, use it verbatim.

## 5. Video Decoding Pipeline

- Format probing: `ffprobe -v error -select_streams v:0 -show_entries stream=nb_frames,nb_read_packets,width,height,duration,avg_frame_rate -of json <input>`.
  - Frame count `N`: prefer `nb_frames`; fall back to counting decoded frames; fall back to `round(duration * avg_frame_rate)`.
- Decode + analysis: single streaming pass —
  ```
  ffmpeg -v error -i <input> -vf "scale=<sw>:-2" -pix_fmt rgb24 -f rawvideo -
  ```
  - `<sw>` default 64 px wide (configurable via `--sample-width`), aspect preserved, RGB24.
  - Read exactly `scaled_w * scaled_h * 3` bytes per frame from stdout into a reusable buffer; count frames until EOF (use this count as `N` if probing failed).
  - Per frame: run §3 histogram → store one `Rgb8` per frame in a `Vec`. Memory: 3 bytes/frame.
- ffmpeg and ffprobe must be on PATH; missing binaries or non-zero exit → hard error with a clear message.

## 6. Rendering

- Allocate RGBA buffer `canvas = vec![0u8; side*side*4]`.
- Single pass over all pixels; for each pixel compute `(dx, dy)` from center, `r`, and angle `θ` mapped to `[0°, 360°)` with 0° = up, clockwise positive.
- If `r_in < r <= R`, angle outside notch, and `θ` falls in frame `i`'s wedge → write frame `i`'s colour, alpha 255. Else alpha 0.
- Encode with the `png` crate (RGBA8, default compression, no interlacing) and write to output path.

## 7. CLI (clap, derive)

```
strisk <INPUT> [OPTIONS]

Arguments:
  <INPUT>               Path to the input video file

Options:
  -o, --output <PATH>   Output PNG path [default: <input_stem>.strisk.png]
  -r, --radius <PX>     Outer disk radius in px [default: auto, see §4]
      --inner-fraction <F>  Blank center radius as fraction of R [default: 0.25]
      --notch-degrees <DEG>  Width of the start/end notch [default: 4.0]
      --levels <L>      Quantization levels per RGB channel [default: 8]
      --sample-width <W>  Decoded analysis width in px [default: 64]
      --jobs <N>           Parallel ffmpeg decode processes [default: 4]
  -h, --help            Print help
  -V, --version         Print version
```

Validation: `levels >= 2`, `0 < inner_fraction < 1`, `0 <= notch_degrees < 60`, `sample_width >= 8`, `radius >= 16` when given.

## 8. Crates / Dependencies

- `clap` (derive) — CLI
- `png` — PNG encoding
- `serde_json` — parse ffprobe JSON
- `thiserror` — error types
- Dev: `tempfile` for tests

No async runtime; std I/O and blocking subprocesses. Target stable Rust.

## 9. Performance Notes

- Release profile: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`.
- Dominant cost is ffmpeg decode; analysis is O(frames × sample_w × sample_h) on a tiny buffer with a fixed 512-entry histogram — no heap allocation per pixel.
- Single streaming decode pass; frame colours stored in a 3-byte-per-frame Vec; render pass is one sweep over the canvas.

## 10. Error Handling

All fallible steps return a typed `StriskError` (thiserror) covering: ffmpeg/ffprobe missing, probe failure, decode short-read, zero frames, invalid CLI values, PNG encode/write failure.

## 11. Testing

- Unit: quantization + histogram mode selection; angle → frame index mapping incl. notch boundaries; inner/outer radius alpha rules.
- Integration: generate a synthetic video with ffmpeg (e.g. two solid-colour halves), run the binary, decode the PNG, assert dimensions, transparency of notch/corners, and that distinct wedge colours appear.

## 12. Out of Scope (v1)

- Anti-aliased edges/supersampling
- Cropping/zooming of input, subtitles, embedded metadata
- Radial animations/GIF output, GUI
- Pure-Rust video decoding
