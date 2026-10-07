#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Reusable scratch buffers for per-frame dominant-colour analysis.
pub struct Analyzer {
    levels: usize,
    counts: Vec<u32>,
    sum: Vec<[u64; 3]>,
}

impl Analyzer {
    pub fn new(levels: usize) -> Self {
        let l = levels.max(2);
        Analyzer {
            levels: l,
            counts: vec![0; l * l * l],
            sum: vec![[0; 3]; l * l * l],
        }
    }

    /// Quantized mode of the frame; representative = mean of original
    /// pixels in the winning bucket.
    pub fn dominant(&mut self, frame: &[u8]) -> Rgb8 {
        debug_assert_eq!(frame.len() % 3, 0);
        let l = self.levels;
        self.counts.fill(0);
        self.sum.fill([0; 3]);
        for px in frame.chunks_exact(3) {
            let idx = (px[0] as usize * l / 256 * l + px[1] as usize * l / 256) * l
                + px[2] as usize * l / 256;
            self.counts[idx] += 1;
            self.sum[idx][0] += px[0] as u64;
            self.sum[idx][1] += px[1] as u64;
            self.sum[idx][2] += px[2] as u64;
        }
        let mut best = 0usize;
        let mut best_c = 0u32;
        for (i, &c) in self.counts.iter().enumerate() {
            if c > best_c {
                best = i;
                best_c = c;
            }
        }
        let n = best_c.max(1) as u64;
        Rgb8 {
            r: (self.sum[best][0] / n) as u8,
            g: (self.sum[best][1] / n) as u8,
            b: (self.sum[best][2] / n) as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominant_is_mode_bucket() {
        let frame: Vec<u8> = [10, 10, 200, 20, 20, 210, 15, 15, 190, 250, 0, 0]
            .iter()
            .copied()
            .collect();
        let c = Analyzer::new(8).dominant(&frame);
        assert!(c.b > 150 && c.r < 60, "got {:?}", c);
    }

    #[test]
    fn deterministic_tie_break() {
        let frame: Vec<u8> = [255, 0, 0, 0, 0, 255].iter().copied().collect();
        let a = Analyzer::new(8).dominant(&frame);
        let b = Analyzer::new(8).dominant(&frame);
        assert_eq!(a, b);
    }
}
