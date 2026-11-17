// Planet definitions. Coordinates are planetocentric, east-positive longitude.
// mapScale shrinks the planet horizontally AND vertically so proportions stay
// true: at 1:50 Mars is ~430 km around instead of 21,000 km.

export const PLANETS = {
  mars: {
    id: 'mars',
    name: 'Mars',
    radius: 3389500,
    gravity: 3.71,
    mapScale: 50,
    demSource: 'MGS MOLA MEGDR',
    sky: { zenith: 0x7a5238, horizon: 0xe2b48a },
    fog: { color: 0xd9a77e, near: 4000, far: 42000 },
    stars: false,
    sun: { color: 0xfff0dc, intensity: 2.8, direction: [0.6, 0.55, 0.25] },
    ambient: { sky: 0xe8b590, ground: 0x5a3020, intensity: 1.1 },
    palette: {
      low: [0.36, 0.17, 0.09], high: [0.72, 0.44, 0.26], steep: [0.24, 0.12, 0.08],
      elevationRange: [-6000, 10000],
    },
    dustColor: 0xc58a5a,
    dustDrag: 0.6,
    detail: {
      hillAmp: 12, hillWavelength: 900, bumpAmp: 0.45, bumpWavelength: 40,
      craterScales: [60, 300, 1500], craterChance: 0.3, seed: 4242,
    },
    procedural: { amp: 4000, frequency: 2.5, seed: 9 },
    landmarks: [
      { name: 'Jezero Crater (Perseverance)', lat: 18.38, lon: 77.58 },
      { name: 'Gale Crater (Curiosity)', lat: -5.4, lon: 137.8 },
      { name: 'Olympus Mons', lat: 18.65, lon: -133.8 },
      { name: 'Valles Marineris (Melas Chasma)', lat: -10.5, lon: -72.5 },
      { name: 'Noctis Labyrinthus', lat: -7.0, lon: -102.0 },
      { name: 'Arsia Mons', lat: -8.26, lon: -120.09 },
      { name: 'Hellas Planitia', lat: -42.4, lon: 70.5 },
    ],
  },
  moon: {
    id: 'moon',
    name: 'Moon',
    radius: 1737400,
    gravity: 1.62,
    mapScale: 25,
    demSource: 'LRO LOLA LDEM',
    sky: { zenith: 0x000000, horizon: 0x05060a },
    fog: { color: 0x000000, near: 15000, far: 60000 },
    stars: true,
    sun: { color: 0xffffff, intensity: 3.4, direction: [0.7, 0.3, 0.35] },
    ambient: { sky: 0x2a2f3a, ground: 0x0c0c0c, intensity: 0.5 },
    palette: {
      low: [0.16, 0.16, 0.17], high: [0.55, 0.54, 0.52], steep: [0.42, 0.42, 0.41],
      elevationRange: [-3000, 3000],
    },
    dustColor: 0x9a9a9a,
    dustDrag: 0,
    detail: {
      hillAmp: 8, hillWavelength: 700, bumpAmp: 0.35, bumpWavelength: 32,
      craterScales: [25, 90, 350, 1400], craterChance: 0.55, seed: 1969,
    },
    procedural: { amp: 2500, frequency: 3, seed: 11 },
    landmarks: [
      { name: 'Tranquility Base (Apollo 11)', lat: 0.674, lon: 23.473 },
      { name: 'Hadley Rille (Apollo 15)', lat: 26.13, lon: 3.63 },
      { name: 'Copernicus', lat: 9.62, lon: -20.08 },
      { name: 'Tycho', lat: -43.31, lon: -11.36 },
      { name: 'Aristarchus', lat: 23.7, lon: -47.4 },
      { name: 'Montes Apenninus', lat: 18.9, lon: -3.7 },
      { name: 'Tsiolkovskiy (far side)', lat: -20.4, lon: 129.1 },
      { name: 'Apollo basin (far side)', lat: -36.1, lon: -151.8 },
    ],
  },
};

export function pickPlanet(search = location.search) {
  const id = new URLSearchParams(search).get('planet');
  return PLANETS[id] ?? PLANETS.mars;
}
