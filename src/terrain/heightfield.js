import { createNoise2D } from 'simplex-noise';
import { hash2, mulberry32, smoothstep } from '../util/math.js';

// Crater profile, in units of crater radius. Depth/diameter ~0.16 is shallower
// than real simple craters (~0.2) so they stay drivable; the sharp rim is a ramp.
const CRATER_DEPTH = 0.32;
const CRATER_RIM = 0.07;
const CRATER_REACH = 2; // rim apron extends to 2 radii
const NORMAL_EPSILON = 0.4; // metres; finite-difference step for normals

/**
 * World <-> planet mapping plus the height function used by both the renderer
 * and the physics, so what you see is what you hit.
 * World x = east, world -z = north, 1 unit = 1 metre at map scale.
 */
export class Heightfield {
  constructor(planet, dem) {
    this.dem = dem;
    this.detail = planet.detail;
    this.mapScale = planet.mapScale;
    this.metresPerDegree = (2 * Math.PI * planet.radius) / 360 / planet.mapScale;
    const rng = mulberry32(planet.detail.seed);
    this.hillNoise = createNoise2D(rng);
    this.bumpNoise = createNoise2D(rng);
  }

  toLatLon(x, z) {
    return { lat: -z / this.metresPerDegree, lon: x / this.metresPerDegree };
  }

  fromLatLon(lat, lon) {
    return { x: lon * this.metresPerDegree, z: -lat * this.metresPerDegree };
  }

  /** True planetary elevation in metres (unscaled), from the DEM alone. */
  elevationAt(x, z) {
    return this.dem.sample(x / this.metresPerDegree, -z / this.metresPerDegree);
  }

  heightAt(x, z) {
    return this.elevationAt(x, z) / this.mapScale + this.detailAt(x, z);
  }

  normalAt(x, z, out) {
    const e = NORMAL_EPSILON;
    const dx = this.heightAt(x - e, z) - this.heightAt(x + e, z);
    const dz = this.heightAt(x, z - e) - this.heightAt(x, z + e);
    return out.set(dx, 2 * e, dz).normalize();
  }

  /** Procedural sub-DEM detail: rolling hills, bumps and craters. */
  detailAt(x, z) {
    const d = this.detail;
    const hx = x / d.hillWavelength, hz = z / d.hillWavelength;
    const hills = this.hillNoise(hx, hz) + 0.5 * this.hillNoise(hx * 2.3, hz * 2.3);
    const bumps = this.bumpNoise(x / d.bumpWavelength, z / d.bumpWavelength);
    return hills * d.hillAmp + bumps * d.bumpAmp + this.cratersAt(x, z);
  }

  cratersAt(x, z) {
    let h = 0;
    const scales = this.detail.craterScales;
    for (let k = 0; k < scales.length; k++) h += this.cratersAtScale(x, z, scales[k], k);
    return h;
  }

  // One crater candidate per cell; check the 3x3 neighbourhood since a
  // crater's apron can spill into adjacent cells.
  cratersAtScale(x, z, cell, salt) {
    const ci = Math.floor(x / cell), cj = Math.floor(z / cell);
    const seed = this.detail.seed + salt * 101;
    let h = 0;
    for (let j = cj - 1; j <= cj + 1; j++) {
      for (let i = ci - 1; i <= ci + 1; i++) {
        if (hash2(i, j, seed) > this.detail.craterChance) continue;
        const radius = cell * (0.12 + 0.3 * hash2(i, j, seed + 3));
        const cx = (i + hash2(i, j, seed + 1)) * cell, cz = (j + hash2(i, j, seed + 2)) * cell;
        const d = Math.hypot(x - cx, z - cz) / radius;
        if (d < CRATER_REACH) h += craterProfile(d) * radius;
      }
    }
    return h;
  }
}

function craterProfile(d) {
  if (d < 1) return CRATER_DEPTH * (d * d - 1) + CRATER_RIM;
  return CRATER_RIM * (1 - smoothstep(1, CRATER_REACH, d));
}
