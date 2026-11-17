//! The height function shared by rendering and physics, so what you see is
//! what you hit. World x = east, world -z = north, 1 unit = 1 m at map scale.

use std::sync::Arc;

use bevy::math::DVec3;
use noise::{NoiseFn, OpenSimplex};

use super::dem::Dem;
use crate::math::{hash2, smoothstep};
use crate::planet::{DetailParams, Planet};

// Crater profile in units of crater radius. Depth/diameter ~0.16 is shallower
// than real simple craters (~0.2) so they stay drivable; the sharp rim is a ramp.
const CRATER_DEPTH: f64 = 0.32;
const CRATER_RIM: f64 = 0.07;
const CRATER_REACH: f64 = 2.0; // rim apron extends to 2 radii
const NORMAL_EPSILON: f64 = 0.4; // metres; finite-difference step for normals

pub struct Heightfield {
    dem: Arc<dyn Dem>,
    detail: DetailParams,
    map_scale: f64,
    metres_per_degree: f64,
    hill_noise: OpenSimplex,
    bump_noise: OpenSimplex,
}

impl Heightfield {
    pub fn new(planet: &Planet, dem: Arc<dyn Dem>) -> Self {
        let d = &planet.detail;
        Self {
            dem,
            detail: *d,
            map_scale: planet.map_scale,
            metres_per_degree: std::f64::consts::TAU * planet.radius / 360.0 / planet.map_scale,
            hill_noise: OpenSimplex::new(d.seed),
            bump_noise: OpenSimplex::new(d.seed.wrapping_add(1)),
        }
    }

    /// (lat, lon) in degrees for a world position.
    pub fn to_lat_lon(&self, x: f64, z: f64) -> (f64, f64) {
        (-z / self.metres_per_degree, x / self.metres_per_degree)
    }

    /// World (x, z) for a lat/lon in degrees.
    pub fn from_lat_lon(&self, lat: f64, lon: f64) -> (f64, f64) {
        (lon * self.metres_per_degree, -lat * self.metres_per_degree)
    }

    /// True planetary elevation in metres (unscaled), from the DEM alone.
    pub fn elevation_at(&self, x: f64, z: f64) -> f64 {
        let (lat, lon) = self.to_lat_lon(x, z);
        self.dem.sample(lon, lat)
    }

    pub fn height_at(&self, x: f64, z: f64) -> f64 {
        self.elevation_at(x, z) / self.map_scale + self.detail_at(x, z)
    }

    pub fn normal_at(&self, x: f64, z: f64) -> DVec3 {
        let e = NORMAL_EPSILON;
        let dx = self.height_at(x - e, z) - self.height_at(x + e, z);
        let dz = self.height_at(x, z - e) - self.height_at(x, z + e);
        DVec3::new(dx, 2.0 * e, dz).normalize()
    }

    /// Procedural sub-DEM detail: rolling hills, bumps and craters.
    fn detail_at(&self, x: f64, z: f64) -> f64 {
        let d = &self.detail;
        let (hx, hz) = (x / d.hill_wavelength, z / d.hill_wavelength);
        let hills = self.hill_noise.get([hx, hz]) + 0.5 * self.hill_noise.get([hx * 2.3, hz * 2.3]);
        let bumps = self.bump_noise.get([x / d.bump_wavelength, z / d.bump_wavelength]);
        hills * d.hill_amp + bumps * d.bump_amp + self.craters_at(x, z)
    }

    fn craters_at(&self, x: f64, z: f64) -> f64 {
        self.detail.crater_scales.iter().enumerate().map(|(k, &cell)| self.craters_at_scale(x, z, cell, k as u32)).sum()
    }

    // One crater candidate per cell; check the 3x3 neighbourhood since a
    // crater's apron can spill into adjacent cells.
    fn craters_at_scale(&self, x: f64, z: f64, cell: f64, salt: u32) -> f64 {
        let (ci, cj) = ((x / cell).floor() as i64, (z / cell).floor() as i64);
        let seed = self.detail.seed.wrapping_add(salt * 101);
        let mut h = 0.0;
        for j in cj - 1..=cj + 1 {
            for i in ci - 1..=ci + 1 {
                if hash2(i, j, seed) > self.detail.crater_chance {
                    continue;
                }
                let radius = cell * (0.12 + 0.3 * hash2(i, j, seed + 3));
                let cx = (i as f64 + hash2(i, j, seed + 1)) * cell;
                let cz = (j as f64 + hash2(i, j, seed + 2)) * cell;
                let d = (x - cx).hypot(z - cz) / radius;
                if d < CRATER_REACH {
                    h += crater_profile(d) * radius;
                }
            }
        }
        h
    }
}

fn crater_profile(d: f64) -> f64 {
    if d < 1.0 {
        CRATER_DEPTH * (d * d - 1.0) + CRATER_RIM
    } else {
        CRATER_RIM * (1.0 - smoothstep(1.0, CRATER_REACH, d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet::mars;
    use crate::terrain::dem::ProceduralDem;

    fn field() -> Heightfield {
        let planet = mars();
        Heightfield::new(&planet, Arc::new(ProceduralDem::new(&planet.procedural)))
    }

    #[test]
    fn lat_lon_round_trips() {
        let hf = field();
        let (x, z) = hf.from_lat_lon(18.38, 77.58);
        let (lat, lon) = hf.to_lat_lon(x, z);
        assert!((lat - 18.38).abs() < 1e-9 && (lon - 77.58).abs() < 1e-9);
    }

    #[test]
    fn north_is_negative_z() {
        let (_, z) = field().from_lat_lon(10.0, 0.0);
        assert!(z < 0.0);
    }

    #[test]
    fn mars_is_shrunk_to_about_430_km_around() {
        let (x, _) = field().from_lat_lon(0.0, 360.0);
        assert!((x / 1000.0 - 426.0).abs() < 1.0, "lap is {} km", x / 1000.0);
    }

    #[test]
    fn normals_point_up_and_are_unit_length() {
        let hf = field();
        let n = hf.normal_at(1234.5, -987.6);
        assert!(n.y > 0.0);
        assert!((n.length() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn crater_rim_is_highest_point_of_profile() {
        assert!(crater_profile(1.0) > crater_profile(0.0));
        assert!(crater_profile(1.0) > crater_profile(1.5));
        assert_eq!(crater_profile(CRATER_REACH), 0.0);
    }
}
