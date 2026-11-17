import * as THREE from 'three';
import { pickPlanet } from './planets.js';
import { loadDem } from './terrain/dem.js';
import { Heightfield } from './terrain/heightfield.js';
import { TerrainColorizer } from './terrain/colorizer.js';
import { ChunkManager } from './terrain/chunks.js';
import { ROVER_SPEC } from './rover/spec.js';
import { RoverBody } from './rover/body.js';
import { RoverModel } from './rover/model.js';
import { ChaseCamera } from './chase-camera.js';
import { DustTrail } from './dust.js';
import { Sky } from './sky.js';
import { Input } from './input.js';
import { Hud } from './hud.js';
import { TrickTracker } from './tricks/trick-tracker.js';
import { TrickFeedback } from './tricks/trick-feedback.js';
import { sampleFlight } from './tricks/flight-sample.js';

const FIXED_DT = 1 / 300; // physics rate; at 420 m/s that's 1.4 m per step
const MAX_SUBSTEPS = 15;
const MAX_FRAME_DT = 0.1;
const SPAWN_DROP = 3; // metres above ground on teleport/reset
const GRAVITY_SCALES = [1, 2, 4];
const STUCK_UP_THRESHOLD = 0.2, STUCK_SPEED = 3, STUCK_SECONDS = 2.5; // on its roof and going nowhere

function createRenderer() {
  const renderer = new THREE.WebGLRenderer({ antialias: true, logarithmicDepthBuffer: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.setSize(innerWidth, innerHeight);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  document.body.prepend(renderer.domElement);
  return renderer;
}

/** Wires the world together and runs the frame loop. */
class Game {
  constructor(planet, dem, demLabel) {
    this.planet = planet;
    this.terrain = new Heightfield(planet, dem);
    this.renderer = createRenderer();
    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(65, innerWidth / innerHeight, 0.3, 200000);
    this.sky = new Sky(this.scene, planet);
    this.chunks = new ChunkManager(this.scene, this.terrain, new TerrainColorizer(planet, this.terrain));
    this.body = new RoverBody(ROVER_SPEC);
    this.model = new RoverModel(ROVER_SPEC);
    this.scene.add(this.model.group);
    this.dust = new DustTrail(this.scene, planet);
    this.chase = new ChaseCamera(this.camera, this.terrain);
    this.input = new Input();
    this.tricks = new TrickTracker();
    this.trickFeedback = new TrickFeedback();
    this.hud = new Hud({ planet, dem, demLabel, onTeleport: (place) => this.teleport(place) });
    Object.assign(this, { gravityIndex: 0, stuckFor: 0, accumulator: 0, last: performance.now() });
    addEventListener('resize', () => this.resize());
  }

  get gravityScale() { return GRAVITY_SCALES[this.gravityIndex]; }

  start() {
    this.teleport(this.planet.landmarks[0]);
    this.renderer.setAnimationLoop((now) => this.frame(now));
  }

  teleport({ lat, lon }) {
    const { x, z } = this.terrain.fromLatLon(lat, lon);
    this.dropAt(x, z);
  }

  dropAt(x, z) {
    this.body.placeAt(x, this.terrain.heightAt(x, z) + SPAWN_DROP + ROVER_SPEC.restLength, z, this.body.heading);
    this.chase.snap(this.body);
    this.tricks.reset();
  }

  respawnHere() {
    this.dropAt(this.body.position.x, this.body.position.z);
  }

  frame(now) {
    const dt = Math.min((now - this.last) / 1000, MAX_FRAME_DT);
    this.last = now;
    this.handleToggles();
    this.recoverIfStuck(dt);
    const gravity = this.planet.gravity * this.gravityScale;
    this.stepPhysics(dt, gravity);
    this.scoreTricks(dt);
    this.chunks.update(this.body.position.x, this.body.position.z);
    this.model.update(this.body, dt);
    this.dust.emit(this.body, ROVER_SPEC, dt);
    this.dust.update(dt, gravity);
    this.chase.update(this.body, dt);
    this.sky.update(this.camera);
    this.hud.update({ body: this.body, terrain: this.terrain, gravityScale: this.gravityScale, streaming: this.chunks.pendingCount });
    this.renderer.render(this.scene, this.camera);
  }

  // Fixed-rate physics, decoupled from frame rate.
  stepPhysics(dt, gravity) {
    const controls = this.input.state;
    this.accumulator += dt;
    for (let n = 0; this.accumulator >= FIXED_DT && n < MAX_SUBSTEPS; n++, this.accumulator -= FIXED_DT) {
      this.body.step(FIXED_DT, controls, this.terrain, gravity);
    }
    this.accumulator = Math.min(this.accumulator, FIXED_DT);
  }

  scoreTricks(dt) {
    const event = this.tricks.update(dt, sampleFlight(this.body, this.terrain));
    this.trickFeedback.handle(event, this.tricks);
  }

  handleToggles() {
    if (this.input.consume('KeyR')) this.respawnHere();
    if (this.input.consume('KeyG')) this.gravityIndex = (this.gravityIndex + 1) % GRAVITY_SCALES.length;
    if (this.input.consume('KeyH')) document.getElementById('help').classList.toggle('hidden');
  }

  recoverIfStuck(dt) {
    const stuck = this.body.up.y < STUCK_UP_THRESHOLD && this.body.speed < STUCK_SPEED;
    this.stuckFor = stuck ? this.stuckFor + dt : 0;
    if (this.stuckFor > STUCK_SECONDS) this.respawnHere();
  }

  resize() {
    this.camera.aspect = innerWidth / innerHeight;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(innerWidth, innerHeight);
  }
}

async function main() {
  const planet = pickPlanet();
  const { dem, label } = await loadDem(planet);
  const game = new Game(planet, dem, label);
  game.start();
  window.game = game; // console poking
}

main().catch((err) => {
  document.getElementById('fatal').textContent = `Failed to start: ${err.message}`;
  throw err;
});
