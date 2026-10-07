use std::io::BufWriter;
use std::path::Path;

use png::{AdaptiveFilterType, BitDepth, ColorType, Compression, Encoder, FilterType};

use crate::error::StriskError;

pub fn write_png(path: &Path, rgba: &[u8], side: u32) -> Result<(), StriskError> {
    let file = std::fs::File::create(path)?;
    let w = BufWriter::new(file);
    let mut enc = Encoder::new(w, side, side);
    enc.set_color(ColorType::Rgba);
    enc.set_depth(BitDepth::Eight);
    enc.set_compression(Compression::Best);
    // Adaptive picks the best row filter (Sub/Up/Avg/Paeth) per scanline,
    // which massively helps the flat-colour radial content.
    enc.set_filter(FilterType::Paeth);
    enc.set_adaptive_filter(AdaptiveFilterType::Adaptive);
    let mut writer = enc.write_header().map_err(|e| StriskError::Png(e.to_string()))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| StriskError::Png(e.to_string()))?;
    Ok(())
}
