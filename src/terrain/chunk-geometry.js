import * as THREE from 'three';

const SKIRT_STEPS = 2; // skirt depth in vertex spacings; hides LOD seams

/**
 * Builds one square terrain tile: a (res+1)^2 grid in chunk-local x/z with
 * world-space y, plus a skirt hanging down from the border to hide cracks
 * against neighbours of a different level of detail.
 */
export function buildChunkGeometry(terrain, colorizer, originX, originZ, size, res) {
  const n = res + 1, step = size / res;
  const heights = sampleHeights(terrain, originX, originZ, step, n + 2);
  const perimeter = perimeterIndices(res);
  const count = n * n + perimeter.length;
  const buffers = { position: new Float32Array(count * 3), normal: new Float32Array(count * 3), color: new Float32Array(count * 3) };

  fillSurface(buffers, heights, { terrain, colorizer, originX, originZ, step, n });
  fillSkirt(buffers, perimeter, n * n, step * SKIRT_STEPS + 1);

  const geometry = new THREE.BufferGeometry();
  for (const [name, array] of Object.entries(buffers)) geometry.setAttribute(name, new THREE.BufferAttribute(array, 3));
  geometry.setIndex(new THREE.BufferAttribute(buildIndices(res, perimeter, n * n), 1));
  geometry.computeBoundingSphere();
  return geometry;
}

// Heights on a grid with a one-sample border so normals are seamless across tiles.
function sampleHeights(terrain, originX, originZ, step, b) {
  const heights = new Float32Array(b * b);
  for (let j = 0; j < b; j++) {
    for (let i = 0; i < b; i++) heights[j * b + i] = terrain.heightAt(originX + (i - 1) * step, originZ + (j - 1) * step);
  }
  return heights;
}

function fillSurface({ position, normal, color }, heights, { colorizer, originX, originZ, step, n }) {
  const b = n + 2, nrm = new THREE.Vector3();
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < n; i++) {
      const h = (j + 1) * b + (i + 1), v = (j * n + i) * 3;
      nrm.set(heights[h - 1] - heights[h + 1], 2 * step, heights[h - b] - heights[h + b]).normalize();
      position[v] = i * step; position[v + 1] = heights[h]; position[v + 2] = j * step;
      normal[v] = nrm.x; normal[v + 1] = nrm.y; normal[v + 2] = nrm.z;
      colorizer.colorAt(originX + i * step, originZ + j * step, nrm.y, color, v);
    }
  }
}

function fillSkirt({ position, normal, color }, perimeter, firstSkirt, drop) {
  perimeter.forEach((src, k) => {
    const s = src * 3, d = (firstSkirt + k) * 3;
    for (let c = 0; c < 3; c++) { position[d + c] = position[s + c]; normal[d + c] = normal[s + c]; color[d + c] = color[s + c]; }
    position[d + 1] -= drop;
  });
}

// Border vertices as a closed loop: top, right, bottom, left.
function perimeterIndices(res) {
  const n = res + 1, loop = [];
  for (let i = 0; i < res; i++) loop.push(i);
  for (let j = 0; j < res; j++) loop.push(j * n + res);
  for (let i = res; i > 0; i--) loop.push(res * n + i);
  for (let j = res; j > 0; j--) loop.push(j * n);
  return loop;
}

function buildIndices(res, perimeter, firstSkirt) {
  const n = res + 1, out = [];
  for (let j = 0; j < res; j++) {
    for (let i = 0; i < res; i++) {
      const a = j * n + i;
      out.push(a, a + n, a + 1, a + 1, a + n, a + n + 1);
    }
  }
  perimeter.forEach((top, k) => {
    const next = (k + 1) % perimeter.length;
    const bottom = firstSkirt + k, nextBottom = firstSkirt + next;
    out.push(top, bottom, perimeter[next], perimeter[next], bottom, nextBottom);
  });
  return new Uint16Array(out);
}
