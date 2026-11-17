import * as THREE from 'three';
import { buildChunkGeometry } from './chunk-geometry.js';

const BASE_CHUNK_SIZE = 64; // metres; finest tile edge (2 m vertex spacing at CHUNK_RES)
const CHUNK_RES = 32; // quads per tile edge
const MAX_LEVEL = 9; // root tiles are 64 m * 2^9 = ~33 km
const ROOT_RING = 1; // 3x3 root tiles around the player
const SPLIT_DISTANCE = 1.0; // split a tile when the player is within one tile-width of it
const BUILD_BUDGET_MS = 6; // per frame; keeps 60 fps while streaming
const MAX_CACHED = 900;
const CHILD_OFFSETS = [[0, 0], [1, 0], [0, 1], [1, 1]];

const sizeOf = (level) => BASE_CHUNK_SIZE * 2 ** level;

/**
 * Quadtree level-of-detail terrain streamed around the player. Tiles are built
 * coarse-first within a per-frame budget; while a fine tile is missing its
 * nearest built ancestor stands in, so there are no holes.
 */
export class ChunkManager {
  constructor(scene, terrain, colorizer) {
    Object.assign(this, { scene, terrain, colorizer });
    this.material = new THREE.MeshLambertMaterial({ vertexColors: true, side: THREE.DoubleSide });
    this.chunks = new Map();
    this.visible = [];
    this.frame = 0;
  }

  update(px, pz) {
    this.frame++;
    const roots = this.buildRoots(px, pz);
    this.buildMissing(roots, px, pz);
    this.visible.forEach((chunk) => (chunk.mesh.visible = false));
    this.visible = [];
    roots.forEach((root) => this.show(root));
    this.evict();
  }

  get pendingCount() { return this.lastPending ?? 0; }

  buildRoots(px, pz) {
    const size = sizeOf(MAX_LEVEL);
    const cx = Math.floor(px / size), cz = Math.floor(pz / size), roots = [];
    for (let dz = -ROOT_RING; dz <= ROOT_RING; dz++) {
      for (let dx = -ROOT_RING; dx <= ROOT_RING; dx++) roots.push(this.buildTree(MAX_LEVEL, cx + dx, cz + dz, px, pz));
    }
    return roots;
  }

  buildTree(level, ix, iz, px, pz) {
    const node = { level, ix, iz, key: `${level}:${ix}:${iz}`, children: null, displayable: undefined };
    if (level > 0 && distanceToTile(node, px, pz) < SPLIT_DISTANCE * sizeOf(level)) {
      node.children = CHILD_OFFSETS.map(([dx, dz]) => this.buildTree(level - 1, ix * 2 + dx, iz * 2 + dz, px, pz));
    }
    return node;
  }

  buildMissing(roots, px, pz) {
    const missing = [];
    const walk = (node) => (node.children ? node.children.forEach(walk) : !this.chunks.has(node.key) && missing.push(node));
    roots.forEach(walk);
    missing.sort((a, b) => b.level - a.level || distanceToTile(a, px, pz) - distanceToTile(b, px, pz));
    this.lastPending = missing.length;
    const deadline = performance.now() + BUILD_BUDGET_MS;
    for (const node of missing) {
      this.buildChunk(node);
      if (performance.now() > deadline) break;
    }
  }

  buildChunk({ level, ix, iz, key }) {
    const size = sizeOf(level), x = ix * size, z = iz * size;
    const geometry = buildChunkGeometry(this.terrain, this.colorizer, x, z, size, CHUNK_RES);
    const mesh = new THREE.Mesh(geometry, this.material);
    mesh.position.set(x, 0, z);
    mesh.visible = false;
    mesh.matrixAutoUpdate = false;
    mesh.updateMatrix();
    this.scene.add(mesh);
    this.chunks.set(key, { mesh, lastUsed: this.frame });
  }

  canDisplay(node) {
    if (node.displayable === undefined) {
      node.displayable = this.chunks.has(node.key) || (node.children !== null && node.children.every((c) => this.canDisplay(c)));
    }
    return node.displayable;
  }

  show(node) {
    if (node.children && node.children.every((c) => this.canDisplay(c))) return node.children.forEach((c) => this.show(c));
    const chunk = this.chunks.get(node.key);
    if (chunk) return this.markVisible(chunk);
    node.children?.forEach((c) => this.show(c));
  }

  markVisible(chunk) {
    chunk.mesh.visible = true;
    chunk.lastUsed = this.frame;
    this.visible.push(chunk);
  }

  evict() {
    if (this.chunks.size <= MAX_CACHED) return;
    const stale = [...this.chunks].filter(([, c]) => c.lastUsed < this.frame).sort((a, b) => a[1].lastUsed - b[1].lastUsed);
    const excess = this.chunks.size - Math.floor(MAX_CACHED * 0.8);
    for (const [key, chunk] of stale.slice(0, excess)) {
      this.scene.remove(chunk.mesh);
      chunk.mesh.geometry.dispose();
      this.chunks.delete(key);
    }
  }
}

// Chebyshev distance from the player to the tile's square footprint.
function distanceToTile({ level, ix, iz }, px, pz) {
  const size = sizeOf(level), half = size / 2;
  const dx = Math.abs(px - (ix + 0.5) * size) - half, dz = Math.abs(pz - (iz + 0.5) * size) - half;
  return Math.max(dx, dz, 0);
}
