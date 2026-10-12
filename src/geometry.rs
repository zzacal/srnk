//! Pure resize arithmetic: the only place the `--factor` percentage becomes pixels.

/// Positive pixel dimensions of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    /// Both components are invariant `> 0`.
    pub fn new(width: u32, height: u32) -> Self {
        debug_assert!(width > 0 && height > 0, "dimensions must be positive");
        Self { width, height }
    }
}

/// `sqrt(factor / 100)` -- the per-dimension scale implied by an area percentage.
pub fn scale_for(factor: u8) -> f64 {
    (f64::from(factor) / 100.0).sqrt()
}

/// `out = max(1, round(dim * sqrt(factor / 100)))` for each dimension.
pub fn resized(source: Dimensions, factor: u8) -> Dimensions {
    let scale = scale_for(factor);
    Dimensions {
        width: scale_dim(source.width, scale),
        height: scale_dim(source.height, scale),
    }
}

fn scale_dim(dim: u32, scale: f64) -> u32 {
    // `f64::round` is half-away-from-zero: the single consistent rule (FR-008a).
    let scaled = (f64::from(dim) * scale).round();
    if scaled < 1.0 {
        1
    } else {
        scaled as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dim(w: u32, h: u32) -> Dimensions {
        Dimensions::new(w, h)
    }

    #[test]
    fn quickstart_numeric_table() {
        assert_eq!(resized(dim(2000, 1500), 25), dim(1000, 750));
        assert_eq!(resized(dim(2000, 1500), 50), dim(1414, 1061));
        assert_eq!(resized(dim(1000, 1000), 99), dim(995, 995));
        assert_eq!(resized(dim(3, 1), 1), dim(1, 1));
    }

    #[test]
    fn repeated_calls_are_identical() {
        for factor in 1..=99u8 {
            let first = resized(dim(1234, 567), factor);
            for _ in 0..5 {
                assert_eq!(resized(dim(1234, 567), factor), first);
            }
        }
    }

    /// SC-003: for real photo sources (well above 100x100) the output pixel count stays
    /// within 1% of the requested percentage. Integer rounding dominates only once an
    /// output dimension falls to a few dozen pixels, which this range excludes.
    #[test]
    fn output_area_is_within_one_percent_of_requested() {
        let sources = [
            dim(4000, 3000),
            dim(2000, 1500),
            dim(1600, 1200),
            dim(1024, 768),
        ];
        for source in sources {
            for factor in 1..=99u8 {
                let out = resized(source, factor);
                let source_area = f64::from(source.width) * f64::from(source.height);
                let out_area = f64::from(out.width) * f64::from(out.height);
                let requested = source_area * f64::from(factor) / 100.0;
                let error = (out_area - requested).abs() / requested;
                assert!(
                    error <= 0.01,
                    "{source:?} @{factor}: got {out:?}, area error {error}"
                );
            }
        }
    }

    #[test]
    fn never_scales_up_and_never_reaches_zero() {
        for factor in 1..=99u8 {
            let out = resized(dim(50, 20), factor);
            assert!(out.width >= 1 && out.height >= 1);
            assert!(out.width <= 50 && out.height <= 20);
        }
    }
}
