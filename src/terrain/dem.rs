//! Planet-scale elevation: real NASA grids when present, noise otherwise.

use std::path::Path;

use noise::{NoiseFn, OpenSimplex};

use crate::planet::ProceduralParams;

/// Elevation in metres for a planetocentric lat/lon (degrees, east-positive).
pub trait Dem: Send + Sync {
    fn sample(&self, lon: f64, lat: f64) -> f64;
}

/// Equirectangular grid from tools/convert_dem.py: little-endian i16 DN, lat
/// 90 at the top row, `scale` metres per DN.
pub struct GridDem {
    width: usize,
    height: usize,
    ppd: f64,
    lon0: f64,
    scale: f64,
    data: Vec<i16>,
}

#[derive(Debug, PartialEq)]
pub struct GridMeta {
    pub width: usize,
    pub height: usize,
    pub ppd: f64,
    pub lon0: f64,
    pub scale: f64,
}

impl GridDem {
    pub fn new(meta: GridMeta, data: Vec<i16>) -> Result<Self, String> {
        if data.len() != meta.width * meta.height {
            return Err(format!("expected {} samples, got {}", meta.width * meta.height, data.len()));
        }
        Ok(Self { width: meta.width, height: meta.height, ppd: meta.ppd, lon0: meta.lon0, scale: meta.scale, data })
    }

    pub fn load(dir: &Path, planet_id: &str) -> Result<Self, String> {
        let meta_text = std::fs::read_to_string(dir.join(format!("{planet_id}.json"))).map_err(|e| e.to_string())?;
        let bytes = std::fs::read(dir.join(format!("{planet_id}.bin"))).map_err(|e| e.to_string())?;
        let data = bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
        Self::new(parse_meta(&meta_text)?, data)
    }

    pub fn pixels_per_degree(&self) -> f64 {
        self.ppd
    }

    fn at(&self, i: usize, j: usize) -> f64 {
        f64::from(self.data[j * self.width + i])
    }
}

impl Dem for GridDem {
    /// Bilinear, wrapping in longitude, clamped at the poles.
    fn sample(&self, lon: f64, lat: f64) -> f64 {
        let (w, h) = (self.width as f64, self.height as f64);
        let u = (lon - self.lon0).rem_euclid(360.0) * self.ppd - 0.5;
        let v = ((90.0 - lat) * self.ppd - 0.5).clamp(0.0, h - 1.0);
        let (i0, j0) = (u.floor(), v.floor());
        let (fu, fv) = (u - i0, v - j0);
        let a = i0.rem_euclid(w) as usize;
        let b = (a + 1) % self.width;
        let (r0, r1) = (j0 as usize, (j0 as usize + 1).min(self.height - 1));
        let top = self.at(a, r0) + (self.at(b, r0) - self.at(a, r0)) * fu;
        let bottom = self.at(a, r1) + (self.at(b, r1) - self.at(a, r1)) * fu;
        (top + (bottom - top) * fv) * self.scale
    }
}

/// The JSON is five flat numeric fields we write ourselves; a full JSON
/// dependency isn't worth it for that.
pub fn parse_meta(text: &str) -> Result<GridMeta, String> {
    let field = |name: &str| -> Result<f64, String> {
        let key = format!("\"{name}\"");
        let start = text.find(&key).ok_or(format!("missing {name}"))? + key.len();
        let rest = text[start..].trim_start().strip_prefix(':').ok_or(format!("bad {name}"))?;
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        rest[..end].trim().parse().map_err(|_| format!("bad number for {name}"))
    };
    Ok(GridMeta {
        width: field("width")? as usize,
        height: field("height")? as usize,
        ppd: field("ppd")?,
        lon0: field("lon0").unwrap_or(0.0),
        scale: field("scale").unwrap_or(1.0),
    })
}

/// Stand-in planet: noise on the unit sphere, so it's seamless at the
/// antimeridian and the poles.
pub struct ProceduralDem {
    amp: f64,
    frequency: f64,
    noise: OpenSimplex,
}

const PROCEDURAL_OCTAVES: usize = 5;

impl ProceduralDem {
    pub fn new(params: &ProceduralParams) -> Self {
        Self { amp: params.amp, frequency: params.frequency, noise: OpenSimplex::new(params.seed) }
    }
}

impl Dem for ProceduralDem {
    fn sample(&self, lon: f64, lat: f64) -> f64 {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        let p = [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()].map(|c| c * self.frequency);
        let (mut sum, mut amp, mut scale) = (0.0, 0.55, 1.0);
        for _ in 0..PROCEDURAL_OCTAVES {
            sum += self.noise.get(p.map(|c| c * scale)) * amp;
            amp *= 0.5;
            scale *= 2.1;
        }
        sum * self.amp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> GridDem {
        // 4x2 grid, 1 px per 90 degrees.
        let meta = GridMeta { width: 4, height: 2, ppd: 4.0 / 360.0, lon0: 0.0, scale: 0.5 };
        GridDem::new(meta, vec![0, 100, 200, 300, 400, 500, 600, 700]).unwrap()
    }

    #[test]
    fn samples_pixel_centres_exactly_with_scale() {
        let dem = grid();
        assert_eq!(dem.sample(45.0, 45.0), 0.0);
        assert_eq!(dem.sample(135.0, 45.0), 50.0);
        assert_eq!(dem.sample(135.0, -45.0), 250.0);
    }

    #[test]
    fn wraps_across_the_antimeridian() {
        let dem = grid();
        assert_eq!(dem.sample(315.0, 45.0), dem.sample(-45.0, 45.0));
        // Halfway between the last and first column.
        assert_eq!(dem.sample(0.0, 45.0), (150.0 + 0.0) / 2.0);
    }

    #[test]
    fn clamps_beyond_the_poles() {
        let dem = grid();
        assert_eq!(dem.sample(45.0, 89.9), dem.sample(45.0, 45.0));
    }

    #[test]
    fn rejects_size_mismatch() {
        let meta = GridMeta { width: 4, height: 2, ppd: 1.0, lon0: 0.0, scale: 1.0 };
        assert!(GridDem::new(meta, vec![0; 7]).is_err());
    }

    #[test]
    fn parses_converter_output() {
        let meta = parse_meta("{\n  \"width\": 1440,\n  \"height\": 720,\n  \"ppd\": 4.0,\n  \"lon0\": 0.0,\n  \"scale\": 0.5\n}").unwrap();
        assert_eq!(meta, GridMeta { width: 1440, height: 720, ppd: 4.0, lon0: 0.0, scale: 0.5 });
    }

    #[test]
    fn procedural_is_seamless_at_the_antimeridian() {
        let dem = ProceduralDem::new(&ProceduralParams { amp: 1000.0, frequency: 2.0, seed: 1 });
        assert!((dem.sample(180.0, 10.0) - dem.sample(-180.0, 10.0)).abs() < 1e-9);
    }
}
