//! Planet definitions. Coordinates are planetocentric, east-positive longitude.
//! `map_scale` shrinks the planet horizontally and vertically so proportions
//! stay true: at 1:50 Mars is ~430 km around instead of 21,000 km.

use bevy::prelude::*;

#[derive(Clone, Copy)]
pub struct Palette {
    pub low: [f32; 3],
    pub high: [f32; 3],
    pub steep: [f32; 3],
    pub elevation_range: (f64, f64),
}

#[derive(Clone, Copy)]
pub struct DetailParams {
    pub hill_amp: f64,
    pub hill_wavelength: f64,
    pub bump_amp: f64,
    pub bump_wavelength: f64,
    pub crater_scales: &'static [f64],
    pub crater_chance: f64,
    pub seed: u32,
}

pub struct ProceduralParams {
    pub amp: f64,
    pub frequency: f64,
    pub seed: u32,
}

pub struct Landmark {
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Resource)]
pub struct Planet {
    pub id: &'static str,
    pub name: &'static str,
    pub radius: f64,
    pub gravity: f64,
    pub map_scale: f64,
    pub dem_source: &'static str,
    pub sky: Color,
    pub fog: (Color, f32, f32),
    pub sun_illuminance: f32,
    pub sun_direction: Vec3,
    pub ambient: (Color, f32),
    pub palette: Palette,
    pub detail: DetailParams,
    pub procedural: ProceduralParams,
    pub landmarks: &'static [Landmark],
}

impl Planet {
    pub fn by_id(id: &str) -> Option<Planet> {
        match id {
            "mars" => Some(mars()),
            "moon" => Some(moon()),
            _ => None,
        }
    }
}

pub fn mars() -> Planet {
    Planet {
        id: "mars",
        name: "Mars",
        radius: 3_389_500.0,
        gravity: 3.71,
        map_scale: 50.0,
        dem_source: "MGS MOLA MEGDR",
        sky: Color::srgb_u8(0xd9, 0xa7, 0x7e),
        fog: (Color::srgb_u8(0xd9, 0xa7, 0x7e), 4_000.0, 42_000.0),
        sun_illuminance: 4_500.0,
        sun_direction: Vec3::new(0.6, 0.55, 0.25),
        ambient: (Color::srgb_u8(0xe8, 0xb5, 0x90), 250.0),
        palette: Palette {
            low: [0.36, 0.17, 0.09],
            high: [0.72, 0.44, 0.26],
            steep: [0.24, 0.12, 0.08],
            elevation_range: (-6_000.0, 10_000.0),
        },
        detail: DetailParams {
            hill_amp: 12.0,
            hill_wavelength: 900.0,
            bump_amp: 0.45,
            bump_wavelength: 40.0,
            crater_scales: &[60.0, 300.0, 1_500.0],
            crater_chance: 0.3,
            seed: 4242,
        },
        procedural: ProceduralParams { amp: 4_000.0, frequency: 2.5, seed: 9 },
        landmarks: &[
            Landmark { name: "Jezero Crater (Perseverance)", lat: 18.38, lon: 77.58 },
            Landmark { name: "Gale Crater (Curiosity)", lat: -5.4, lon: 137.8 },
            Landmark { name: "Olympus Mons", lat: 18.65, lon: -133.8 },
            Landmark { name: "Valles Marineris (Melas Chasma)", lat: -10.5, lon: -72.5 },
            Landmark { name: "Noctis Labyrinthus", lat: -7.0, lon: -102.0 },
            Landmark { name: "Arsia Mons", lat: -8.26, lon: -120.09 },
            Landmark { name: "Hellas Planitia", lat: -42.4, lon: 70.5 },
        ],
    }
}

pub fn moon() -> Planet {
    Planet {
        id: "moon",
        name: "Moon",
        radius: 1_737_400.0,
        gravity: 1.62,
        map_scale: 25.0,
        dem_source: "LRO LOLA LDEM",
        sky: Color::BLACK,
        fog: (Color::BLACK, 15_000.0, 60_000.0),
        sun_illuminance: 7_000.0,
        sun_direction: Vec3::new(0.7, 0.3, 0.35),
        ambient: (Color::srgb_u8(0x2a, 0x2f, 0x3a), 120.0),
        palette: Palette {
            low: [0.16, 0.16, 0.17],
            high: [0.55, 0.54, 0.52],
            steep: [0.42, 0.42, 0.41],
            elevation_range: (-3_000.0, 3_000.0),
        },
        detail: DetailParams {
            hill_amp: 8.0,
            hill_wavelength: 700.0,
            bump_amp: 0.35,
            bump_wavelength: 32.0,
            crater_scales: &[25.0, 90.0, 350.0, 1_400.0],
            crater_chance: 0.55,
            seed: 1969,
        },
        procedural: ProceduralParams { amp: 2_500.0, frequency: 3.0, seed: 11 },
        landmarks: &[
            Landmark { name: "Tranquility Base (Apollo 11)", lat: 0.674, lon: 23.473 },
            Landmark { name: "Hadley Rille (Apollo 15)", lat: 26.13, lon: 3.63 },
            Landmark { name: "Copernicus", lat: 9.62, lon: -20.08 },
            Landmark { name: "Tycho", lat: -43.31, lon: -11.36 },
            Landmark { name: "Aristarchus", lat: 23.7, lon: -47.4 },
            Landmark { name: "Montes Apenninus", lat: 18.9, lon: -3.7 },
            Landmark { name: "Tsiolkovskiy (far side)", lat: -20.4, lon: 129.1 },
            Landmark { name: "Apollo basin (far side)", lat: -36.1, lon: -151.8 },
        ],
    }
}
