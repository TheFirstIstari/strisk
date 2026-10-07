use rayon::prelude::*;

use crate::analyze::Rgb8;

pub struct RenderParams {
    pub radius: u32,
    pub inner_fraction: f64,
    pub notch_degrees: f64,
}

pub const MAX_SIDE: u32 = 8192;

pub fn auto_radius(frame_count: usize, inner_fraction: f64, notch_degrees: f64) -> u32 {
    let notch_rad = notch_degrees.to_radians();
    let delta = (2.0 * std::f64::consts::PI - notch_rad) / frame_count.max(1) as f64;
    let r = (1.0 / (delta * inner_fraction)).ceil() as u32;
    let max_r = MAX_SIDE.saturating_sub(2) / 2;
    r.clamp(16, max_r)
}

/// Angle in degrees, 0 = up, clockwise, range [0, 360).
fn pixel_angle(dx: f64, dy: f64) -> f64 {
    dx.atan2(-dy).to_degrees().rem_euclid(360.0)
}

pub fn frame_index(angle_deg: f64, frame_count: usize, notch_degrees: f64) -> Option<usize> {
    let half = notch_degrees / 2.0;
    if angle_deg < half || angle_deg >= 360.0 - half {
        return None;
    }
    let delta = (360.0 - notch_degrees) / frame_count as f64;
    Some(((angle_deg - half) / delta) as usize % frame_count)
}

/// Fill one RGBA row (`out` must be side*4 bytes, pre-zeroed).
pub fn render_row(colors: &[Rgb8], p: &RenderParams, _side: usize, y: usize, out: &mut [u8]) {
    let center = p.radius as f64 + 1.0;
    let r_in = p.inner_fraction * p.radius as f64;
    let r_in2 = r_in * r_in;
    let r_out2 = (p.radius as f64) * (p.radius as f64);
    let dy = y as f64 + 0.5 - center;
    let dy2 = dy * dy;
    for (x, px) in out.chunks_exact_mut(4).enumerate() {
        let dx = x as f64 + 0.5 - center;
        let r2 = dx * dx + dy2;
        if r2 <= r_in2 || r2 > r_out2 {
            continue;
        }
        if let Some(i) = frame_index(pixel_angle(dx, dy), colors.len(), p.notch_degrees) {
            let c = colors[i];
            px[0] = c.r;
            px[1] = c.g;
            px[2] = c.b;
            px[3] = 255;
        }
    }
}

pub fn render(colors: &[Rgb8], p: &RenderParams) -> Vec<u8> {
    let side = (2 * p.radius + 2) as usize;
    let mut buf = vec![0u8; side * side * 4];
    buf.par_chunks_mut(side * 4)
        .enumerate()
        .for_each(|(y, row)| render_row(colors, p, side, y, row));
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_mapping() {
        assert!((pixel_angle(0.0, -1.0) - 0.0).abs() < 1e-9);
        assert!((pixel_angle(1.0, 0.0) - 90.0).abs() < 1e-9);
        assert!((pixel_angle(0.0, 1.0) - 180.0).abs() < 1e-9);
        assert!((pixel_angle(-1.0, 0.0) - 270.0).abs() < 1e-9);
    }

    #[test]
    fn notch_excluded() {
        assert_eq!(frame_index(0.0, 40, 4.0), None);
        assert_eq!(frame_index(359.9, 40, 4.0), None);
        assert!(frame_index(10.0, 40, 4.0).is_some());
    }

    #[test]
    fn frame_index_progression() {
        let n = 40;
        let first = frame_index(2.5, n, 4.0).unwrap();
        let second = frame_index(2.5 + 356.0 / n as f64, n, 4.0).unwrap();
        assert_eq!(first, 0);
        assert_eq!(second, 1);
    }

    #[test]
    fn auto_radius_satisfies_one_px_arc() {
        let r = auto_radius(40, 0.25, 4.0);
        let delta = (2.0 * std::f64::consts::PI - 4.0f64.to_radians()) / 40.0;
        assert!(delta * r as f64 * 0.25 >= 1.0);
    }

    #[test]
    fn auto_radius_clamped() {
        let r = auto_radius(100_000, 0.25, 4.0);
        assert!(2 * r + 2 <= MAX_SIDE);
    }
}
