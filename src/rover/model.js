import * as THREE from 'three';

const AIR_SPIN_DECAY = 0.4; // 1/s; free-spinning wheels slow down in the air

/** Visual rover: chassis, mast, solar deck and four steerable, sprung wheels. */
export class RoverModel {
  constructor(spec) {
    this.spec = spec;
    this.group = new THREE.Group();
    this.group.add(buildChassis(spec));
    this.wheels = spec.wheelMounts.map(() => this.addWheel(spec));
  }

  addWheel(spec) {
    const pivot = new THREE.Group();
    const geometry = new THREE.CylinderGeometry(spec.wheelRadius, spec.wheelRadius, spec.wheelWidth, 10).rotateZ(Math.PI / 2);
    const tyre = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color: 0x55585c, metalness: 0.6, roughness: 0.5, flatShading: true }));
    pivot.add(tyre);
    this.group.add(pivot);
    return { pivot, tyre, spinRate: 0 };
  }

  update(body, dt) {
    this.group.position.copy(body.position);
    this.group.quaternion.copy(body.quaternion);
    body.wheels.forEach((state, i) => {
      const view = this.wheels[i];
      view.pivot.position.set(state.mount.x, state.mount.y - state.extension, state.mount.z);
      view.pivot.rotation.y = state.steer;
      view.spinRate = state.grounded ? state.groundSpeed / this.spec.wheelRadius : view.spinRate * (1 - AIR_SPIN_DECAY * dt);
      view.tyre.rotation.x -= view.spinRate * dt;
    });
  }
}

function buildChassis({ halfExtents: e }) {
  const chassis = new THREE.Group();
  const add = (geometry, color, x, y, z, options = {}) => {
    const mesh = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color, roughness: 0.6, ...options }));
    mesh.position.set(x, y, z);
    chassis.add(mesh);
    return mesh;
  };
  add(new THREE.BoxGeometry(e.x * 2, e.y * 1.2, e.z * 2), 0xd8d3c6, 0, 0, 0);
  add(new THREE.BoxGeometry(e.x * 1.4, e.y * 0.6, e.z * 1.1), 0xc9a43a, 0, e.y * 0.85, 0.2, { metalness: 0.8, roughness: 0.3 });
  add(new THREE.BoxGeometry(e.x * 2.4, 0.05, e.z * 1.0), 0x1b2a55, 0, e.y * 0.9, e.z * 0.75, { metalness: 0.4, roughness: 0.2 });
  add(new THREE.CylinderGeometry(0.06, 0.06, 1.4), 0xbbbbbb, -e.x * 0.6, e.y + 0.7, -e.z * 0.7);
  add(new THREE.BoxGeometry(0.5, 0.25, 0.3), 0xeeeeee, -e.x * 0.6, e.y + 1.45, -e.z * 0.7);
  add(new THREE.CylinderGeometry(0.35, 0.35, 0.05, 16), 0xdddddd, e.x * 0.6, e.y + 0.35, e.z * 0.1);
  return chassis;
}
