//! Vertex colour from true elevation (maria dark, highlands bright) and slope.

use crate::math::{hash2, lerp, smoothstep};
use crate::planet::Palette;

use super::heightfield::Heightfield;

const STEEP_START: f64 = 0.9; // normal.y where slope tint begins (~25 deg)
const STEEP_FULL: f64 = 0.55; // normal.y for full slope tint (~57 deg)
const JITTER: f64 = 0.06; // per-vertex brightness noise, keeps flat ground from looking plastic
const JITTER_CELL: f64 = 3.0; // metres

pub fn terrain_color(palette: &Palette, heightfield: &Heightfield, x: f64, z: f64, normal_y: f64) -> [f32; 4] {
    let (lo, hi) = palette.elevation_range;
    let t = smoothstep(lo, hi, heightfield.elevation_at(x, z));
    let s = ((STEEP_START - normal_y) / (STEEP_START - STEEP_FULL)).clamp(0.0, 1.0);
    let cell = hash2((x / JITTER_CELL).floor() as i64, (z / JITTER_CELL).floor() as i64, 7);
    let jitter = 1.0 + (cell - 0.5) * 2.0 * JITTER;
    let channel = |c: usize| {
        let base = lerp(f64::from(palette.low[c]), f64::from(palette.high[c]), t);
        (lerp(base, f64::from(palette.steep[c]), s) * jitter) as f32
    };
    [channel(0), channel(1), channel(2), 1.0]
}
