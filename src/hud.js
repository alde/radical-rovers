import { clamp, lerp, smoothstep } from './util/math.js';

const MAP_WIDTH = 360, MAP_HEIGHT = 180; // one pixel per degree
const HILLSHADE_STRENGTH = 12;

const $ = (id) => document.getElementById(id);
const fmt = (n) => Math.round(n).toLocaleString('en-GB');

/** Speedo, position readout, landmark picker and a click-to-teleport minimap. */
export class Hud {
  constructor({ planet, dem, demLabel, onTeleport }) {
    Object.assign(this, { planet, onTeleport });
    $('planet-name').textContent = planet.name;
    $('dem-label').textContent = `${demLabel} · scale 1:${planet.mapScale}`;
    document.querySelectorAll('[data-planet]').forEach((a) => a.classList.toggle('active', a.dataset.planet === planet.id));
    this.overlay = $('minimap-overlay').getContext('2d');
    drawMinimap($('minimap').getContext('2d'), dem, planet.palette);
    this.bindLandmarks(planet.landmarks);
    $('minimap-overlay').addEventListener('click', (e) => this.teleportFromMap(e));
  }

  bindLandmarks(landmarks) {
    const select = $('landmarks');
    landmarks.forEach((l, i) => select.add(new Option(l.name, i)));
    select.addEventListener('change', () => {
      this.onTeleport(landmarks[select.value]);
      select.blur();
    });
  }

  teleportFromMap(event) {
    const rect = event.target.getBoundingClientRect();
    const lon = ((event.clientX - rect.left) / rect.width) * 360 - 180;
    const lat = 90 - ((event.clientY - rect.top) / rect.height) * 180;
    this.onTeleport({ lat, lon });
  }

  update({ body, terrain, gravityScale, streaming }) {
    const { lat, lon } = terrain.toLatLon(body.position.x, body.position.z);
    const kmh = body.speed * 3.6;
    $('speed').textContent = fmt(kmh);
    $('true-speed').textContent = `≈ ${fmt(kmh * this.planet.mapScale)} km/h at true scale`;
    $('elevation').textContent = `${fmt(terrain.elevationAt(body.position.x, body.position.z))} m`;
    $('position').textContent = `${formatLat(lat)}  ${formatLon(lon)}`;
    $('gravity').textContent = gravityScale === 1 ? `${this.planet.gravity} m/s²` : `${this.planet.gravity} × ${gravityScale} m/s²`;
    $('streaming').style.opacity = streaming > 20 ? 1 : 0;
    this.drawMarker(lat, lon, body.heading);
  }

  drawMarker(lat, lon, heading) {
    const ctx = this.overlay;
    const x = ((((lon + 180) % 360) + 360) % 360), y = 90 - clamp(lat, -90, 90);
    ctx.clearRect(0, 0, MAP_WIDTH, MAP_HEIGHT);
    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(-heading);
    ctx.fillStyle = '#ffffff';
    ctx.strokeStyle = '#000000';
    ctx.beginPath();
    ctx.moveTo(0, -7); ctx.lineTo(4.5, 5); ctx.lineTo(-4.5, 5); ctx.closePath();
    ctx.fill(); ctx.stroke();
    ctx.restore();
  }
}

function drawMinimap(ctx, dem, palette) {
  const image = ctx.createImageData(MAP_WIDTH, MAP_HEIGHT);
  const [lo, hi] = dem.range, span = hi - lo;
  const sample = (px, py) => dem.sample(px - 180 + 0.5, 90 - py - 0.5);
  for (let py = 0; py < MAP_HEIGHT; py++) {
    for (let px = 0; px < MAP_WIDTH; px++) {
      const e = sample(px, py);
      const slope = (sample(px - 1, py) - sample(px + 1, py) + sample(px, py - 1) - sample(px, py + 1)) / span;
      const shade = clamp(1 + slope * HILLSHADE_STRENGTH, 0.4, 1.6);
      const t = smoothstep(lo, hi, e), k = (py * MAP_WIDTH + px) * 4;
      for (let c = 0; c < 3; c++) image.data[k + c] = clamp(lerp(palette.low[c], palette.high[c], t) * shade * 1.6, 0, 1) * 255;
      image.data[k + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
}

const formatLat = (lat) => `${Math.abs(lat).toFixed(2)}°${lat >= 0 ? 'N' : 'S'}`;
const formatLon = (lon) => {
  const wrapped = ((((lon + 180) % 360) + 360) % 360) - 180;
  return `${Math.abs(wrapped).toFixed(2)}°${wrapped >= 0 ? 'E' : 'W'}`;
};
