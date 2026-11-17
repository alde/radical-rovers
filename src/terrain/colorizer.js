import { clamp, hash2, lerp, smoothstep } from '../util/math.js';

const STEEP_START = 0.9; // normal.y where slope tint begins (~25 deg)
const STEEP_FULL = 0.55; // normal.y for full slope tint (~57 deg)
const JITTER = 0.06; // per-vertex brightness noise, keeps flat ground from looking plastic
const JITTER_CELL = 3; // metres

/** Vertex colour from true elevation (maria dark, highlands bright) and slope. */
export class TerrainColorizer {
  constructor(planet, terrain) {
    this.palette = planet.palette;
    this.terrain = terrain;
  }

  colorAt(x, z, normalY, out, offset) {
    const { low, high, steep, elevationRange: [lo, hi] } = this.palette;
    const t = smoothstep(lo, hi, this.terrain.elevationAt(x, z));
    const s = clamp((STEEP_START - normalY) / (STEEP_START - STEEP_FULL), 0, 1);
    const j = 1 + (hash2(Math.floor(x / JITTER_CELL), Math.floor(z / JITTER_CELL), 7) - 0.5) * 2 * JITTER;
    for (let c = 0; c < 3; c++) out[offset + c] = lerp(lerp(low[c], high[c], t), steep[c], s) * j;
  }
}
