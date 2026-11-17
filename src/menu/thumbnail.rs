//! Little hillshaded maps of each planet for the title cards.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::math::lerp;
use crate::planet::Palette;
use crate::terrain::Terrain;

pub const WIDTH: usize = 360; // one pixel per degree
pub const HEIGHT: usize = 180;
const HILLSHADE_STRENGTH: f64 = 12.0;
const BRIGHTNESS: f64 = 1.6;

/// RGBA pixels for an equirectangular map, centred on longitude 0.
pub fn thumbnail_rgba(sample: impl Fn(f64, f64) -> f64, palette: &Palette) -> Vec<u8> {
    let heights: Vec<f64> = (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (y, x))).map(|(y, x)| sample(89.5 - y as f64, x as f64 - 179.5)).collect();
    let (lo, hi) = heights.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &h| (lo.min(h), hi.max(h)));
    let span = (hi - lo).max(1.0);
    let at = |x: isize, y: usize| heights[y * WIDTH + x.rem_euclid(WIDTH as isize) as usize];
    let mut rgba = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for y in 0..HEIGHT {
        for x in 0..WIDTH as isize {
            let (up, down) = (y.saturating_sub(1), (y + 1).min(HEIGHT - 1));
            let slope = (at(x - 1, y) - at(x + 1, y) + at(x, up) - at(x, down)) / span;
            let shade = (1.0 + slope * HILLSHADE_STRENGTH).clamp(0.4, 1.6);
            let t = (at(x, y) - lo) / span;
            for c in 0..3 {
                let v = lerp(f64::from(palette.low[c]), f64::from(palette.high[c]), t) * shade * BRIGHTNESS;
                rgba.push((v.clamp(0.0, 1.0) * 255.0) as u8);
            }
            rgba.push(255);
        }
    }
    rgba
}

pub fn thumbnail_image(terrain: &Terrain) -> Image {
    let hf = &terrain.heightfield;
    let pixels = thumbnail_rgba(|lat, lon| {
        let (x, z) = hf.from_lat_lon(lat, lon);
        hf.elevation_at(x, z)
    }, &terrain.palette);
    let size = Extent3d { width: WIDTH as u32, height: HEIGHT as u32, depth_or_array_layers: 1 };
    Image::new(size, TextureDimension::D2, pixels, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::mars;

    #[test]
    fn low_ground_is_darker_than_high_ground() {
        let palette = mars().palette;
        // Northern hemisphere high, southern low.
        let rgba = thumbnail_rgba(|lat, _| if lat > 0.0 { 1000.0 } else { -1000.0 }, &palette);
        assert_eq!(rgba.len(), WIDTH * HEIGHT * 4);
        let red = |y: usize| rgba[(y * WIDTH + 100) * 4];
        assert!(red(20) > red(160));
    }
}
