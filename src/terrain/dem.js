import { createNoise3D } from 'simplex-noise';
import { clamp, mulberry32 } from '../util/math.js';

/**
 * Real elevation grid (equirectangular, lat 90 at the top row).
 * Produced by tools/convert_dem.py from NASA PDS MOLA / LOLA products.
 */
export class GridDem {
  constructor({ width, height, ppd, lon0 = 0, scale = 1 }, data) {
    Object.assign(this, { width, height, ppd, lon0, scale, data });
    this.range = this.computeRange();
  }

  /** Elevation in metres, bilinear, wraps in longitude. */
  sample(lon, lat) {
    const { width: w, height: h, ppd, data } = this;
    const u = ((((lon - this.lon0) % 360) + 360) % 360) * ppd - 0.5;
    const v = clamp((90 - lat) * ppd - 0.5, 0, h - 1);
    const i0 = Math.floor(u), j0 = Math.floor(v);
    const fu = u - i0, fv = v - j0;
    const a = ((i0 % w) + w) % w, b = (a + 1) % w;
    const r0 = j0 * w, r1 = Math.min(j0 + 1, h - 1) * w;
    const top = data[r0 + a] + (data[r0 + b] - data[r0 + a]) * fu;
    const bottom = data[r1 + a] + (data[r1 + b] - data[r1 + a]) * fu;
    return (top + (bottom - top) * fv) * this.scale;
  }

  computeRange() {
    let min = Infinity, max = -Infinity;
    for (const d of this.data) { if (d < min) min = d; if (d > max) max = d; }
    return [min * this.scale, max * this.scale];
  }
}

/** Stand-in planet when no DEM is on disk: noise on the unit sphere, seamless at the poles. */
export class ProceduralDem {
  constructor({ amp, frequency, seed }) {
    this.amp = amp;
    this.frequency = frequency;
    this.noise = createNoise3D(mulberry32(seed));
    this.range = [-amp, amp];
  }

  sample(lon, lat) {
    const la = (lat * Math.PI) / 180, lo = (lon * Math.PI) / 180;
    const f = this.frequency;
    const x = Math.cos(la) * Math.cos(lo) * f, y = Math.cos(la) * Math.sin(lo) * f, z = Math.sin(la) * f;
    let sum = 0, amp = 0.55, scale = 1;
    for (let octave = 0; octave < 5; octave++) {
      sum += this.noise(x * scale, y * scale, z * scale) * amp;
      amp *= 0.5;
      scale *= 2.1;
    }
    return sum * this.amp;
  }
}

export async function loadDem(planet) {
  try {
    const meta = await fetchOk(`data/${planet.id}.json`).then((r) => r.json());
    const buffer = await fetchOk(`data/${planet.id}.bin`).then((r) => r.arrayBuffer());
    const label = `${planet.demSource} ${meta.ppd} px/deg`;
    return { dem: new GridDem(meta, new Int16Array(buffer)), label, real: true };
  } catch (err) {
    console.info(`No DEM for ${planet.id}, using procedural terrain:`, err.message);
    return { dem: new ProceduralDem(planet.procedural), label: 'procedural placeholder', real: false };
  }
}

async function fetchOk(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  return response;
}
