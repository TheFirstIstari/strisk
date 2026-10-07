use std::io::{BufWriter, Write};
use std::path::Path;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::error::StriskError;

const IDAT_SLAB: usize = 8 * 1024 * 1024;

// ---- chunk helpers ----------------------------------------------------

fn crc32_of(parts: &[&[u8]]) -> u32 {
    let mut h = crc32fast::Hasher::new();
    for p in parts {
        h.update(p);
    }
    h.finalize()
}

fn write_chunk<W: Write>(w: &mut W, typ: &[u8; 4], data: &[u8]) -> std::io::Result<()> {
    w.write_all(&(data.len() as u32).to_be_bytes())?;
    w.write_all(typ)?;
    w.write_all(data)?;
    w.write_all(&crc32_of(&[typ, data]).to_be_bytes())?;
    Ok(())
}

/// Collects the zlib stream and emits it as multiple IDAT chunks,
/// so the compressed payload never sits in memory.
struct IdatOut<W: Write> {
    out: W,
    buf: Vec<u8>,
}

impl<W: Write> IdatOut<W> {
    fn new(out: W) -> Self {
        IdatOut {
            out,
            buf: Vec::with_capacity(IDAT_SLAB),
        }
    }

    fn flush_chunk(&mut self) -> std::io::Result<()> {
        if !self.buf.is_empty() {
            write_chunk(&mut self.out, b"IDAT", &self.buf)?;
            self.buf.clear();
        }
        Ok(())
    }

    fn finish_stream(mut self) -> std::io::Result<W> {
        self.flush_chunk()?;
        Ok(self.out)
    }
}

impl<W: Write> Write for IdatOut<W> {
    fn write(&mut self, mut data: &[u8]) -> std::io::Result<usize> {
        let n = data.len();
        while !data.is_empty() {
            let space = IDAT_SLAB - self.buf.len();
            let take = data.len().min(space);
            self.buf.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.buf.len() == IDAT_SLAB {
                self.flush_chunk()?;
            }
        }
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_chunk()?;
        self.out.flush()
    }
}

// ---- PNG row filtering --------------------------------------------------

fn filter_row(filter: u8, row: &[u8], prev: &[u8], stride: usize, scratch: &mut [u8]) {
    match filter {
        0 => scratch.copy_from_slice(row),
        1 => {
            for i in 0..row.len() {
                let a = if i >= stride { row[i - stride] } else { 0 };
                scratch[i] = row[i].wrapping_sub(a);
            }
        }
        2 => {
            for i in 0..row.len() {
                scratch[i] = row[i].wrapping_sub(prev[i]);
            }
        }
        3 => {
            for i in 0..row.len() {
                let a = if i >= stride { row[i - stride] } else { 0 };
                scratch[i] = row[i].wrapping_sub(((a as u16 + prev[i] as u16) / 2) as u8);
            }
        }
        4 => {
            for i in 0..row.len() {
                let a = if i >= stride { row[i - stride] } else { 0 };
                let b = prev[i];
                let c = if i >= stride { prev[i - stride] } else { 0 };
                let p = a as i32 + b as i32 - c as i32;
                let pa = (p - a as i32).abs();
                let pb = (p - b as i32).abs();
                let pc = (p - c as i32).abs();
                let pred = if pa <= pb && pa <= pc {
                    a
                } else if pb <= pc {
                    b
                } else {
                    c
                };
                scratch[i] = row[i].wrapping_sub(pred);
            }
        }
        _ => unreachable!(),
    }
}

fn score(buf: &[u8]) -> u64 {
    buf.iter()
        .map(|&b| (b as i8 as i32).unsigned_abs() as u64)
        .sum()
}

// ---- public API -----------------------------------------------------------

/// Encode an RGBA8 image to PNG, pulling each row from `row_fn(y, buf)`.
/// Memory use is O(row) + O(8MB IDAT slab). Lossless.
pub fn write_png_streaming<F>(
    path: &Path,
    width: u32,
    height: u32,
    mut row_fn: F,
) -> Result<(), StriskError>
where
    F: FnMut(usize, &mut [u8]),
{
    let row_len = (width as usize) * 4;
    let file = std::fs::File::create(path)?;
    let mut w = BufWriter::new(file);

    w.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?; // PNG signature

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA, deflate, adaptive, no interlace
    write_chunk(&mut w, b"IHDR", &ihdr)?;

    let idat = IdatOut::new(&mut w);
    let mut zlib = ZlibEncoder::new(idat, Compression::best());

    let mut row = vec![0u8; row_len];
    let mut prev = vec![0u8; row_len];
    let mut scratch = vec![0u8; row_len];
    for y in 0..height as usize {
        row_fn(y, &mut row);
        let mut best_filter = 0u8;
        let mut best_score = u64::MAX;
        for f in 0u8..5 {
            filter_row(f, &row, &prev, 4, &mut scratch);
            let s = score(&scratch);
            if s < best_score {
                best_score = s;
                best_filter = f;
            }
        }
        filter_row(best_filter, &row, &prev, 4, &mut scratch);
        zlib.write_all(&[best_filter])?;
        zlib.write_all(&scratch)?;
        prev.copy_from_slice(&row);
    }

    let idat = zlib.finish().map_err(|e| StriskError::Png(e.to_string()))?;
    idat.finish_stream()?;

    write_chunk(&mut w, b"IEND", &[])?;
    w.flush()?;
    Ok(())
}
