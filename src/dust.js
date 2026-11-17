import * as THREE from 'three';

const MAX_PARTICLES = 4000;
const LIFETIME = 2.5; // seconds
const PARTICLES_PER_METRE = 0.6; // per wheel
const MIN_SPEED = 4; // m/s before wheels kick up dust
const HIDDEN_Y = -1e7;

const _origin = new THREE.Vector3();

/** Ballistic dust from the wheels: no air on the Moon, so it arcs like a cannonball. */
export class DustTrail {
  constructor(scene, planet) {
    this.drag = planet.dustDrag;
    this.positions = new Float32Array(MAX_PARTICLES * 3).fill(HIDDEN_Y);
    this.velocities = new Float32Array(MAX_PARTICLES * 3);
    this.ages = new Float32Array(MAX_PARTICLES).fill(LIFETIME);
    this.next = 0;
    this.carry = 0;
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(this.positions, 3));
    const material = new THREE.PointsMaterial({ color: planet.dustColor, size: 0.45, transparent: true, opacity: 0.7, depthWrite: false });
    this.points = new THREE.Points(geometry, material);
    this.points.frustumCulled = false;
    scene.add(this.points);
  }

  emit(body, spec, dt) {
    const speed = body.speed;
    if (speed < MIN_SPEED) return;
    for (const wheel of body.wheels) {
      if (!wheel.grounded) continue;
      this.carry += speed * dt * PARTICLES_PER_METRE;
      _origin.copy(wheel.mount).setY(wheel.mount.y - wheel.extension - spec.wheelRadius).applyQuaternion(body.quaternion).add(body.position);
      for (; this.carry >= 1; this.carry--) this.spawn(_origin, body.velocity, speed);
    }
  }

  spawn(origin, velocity, speed) {
    const i = this.next, k = i * 3, kick = 1 + speed * 0.04;
    this.next = (i + 1) % MAX_PARTICLES;
    this.ages[i] = Math.random() * 0.5;
    this.positions[k] = origin.x; this.positions[k + 1] = origin.y; this.positions[k + 2] = origin.z;
    this.velocities[k] = velocity.x * 0.35 + (Math.random() - 0.5) * kick;
    this.velocities[k + 1] = velocity.y * 0.35 + Math.random() * kick;
    this.velocities[k + 2] = velocity.z * 0.35 + (Math.random() - 0.5) * kick;
  }

  update(dt, gravity) {
    const damping = Math.max(0, 1 - this.drag * dt);
    for (let i = 0; i < MAX_PARTICLES; i++) {
      if (this.ages[i] >= LIFETIME) continue;
      const k = i * 3;
      this.ages[i] += dt;
      if (this.ages[i] >= LIFETIME) { this.positions[k + 1] = HIDDEN_Y; continue; }
      this.velocities[k + 1] -= gravity * dt;
      for (let c = 0; c < 3; c++) { this.velocities[k + c] *= damping; this.positions[k + c] += this.velocities[k + c] * dt; }
    }
    this.points.geometry.attributes.position.needsUpdate = true;
  }
}
