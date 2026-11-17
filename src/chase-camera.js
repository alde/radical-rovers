import * as THREE from 'three';

const BASE_DISTANCE = 9, DISTANCE_PER_SPEED = 0.025, MAX_DISTANCE = 22;
const BASE_HEIGHT = 3.2, HEIGHT_PER_SPEED = 0.006;
const BASE_FOV = 65, MAX_EXTRA_FOV = 25, FOV_FULL_SPEED = 400; // speed sensation via FOV widening
const FOLLOW_RATE = 5; // 1/s
const HEADING_RATE = 3; // 1/s
const MIN_CLEARANCE = 1.5;

const _desired = new THREE.Vector3(), _target = new THREE.Vector3(), _dir = new THREE.Vector3();

/**
 * Chase camera smoothed in rover-relative space, so it never trails hundreds
 * of metres behind at boost speed.
 */
export class ChaseCamera {
  constructor(camera, terrain) {
    this.camera = camera;
    this.terrain = terrain;
    this.offset = new THREE.Vector3(0, BASE_HEIGHT, BASE_DISTANCE);
    this.heading = new THREE.Vector3(0, 0, -1);
  }

  snap(body) {
    this.heading.copy(flatForward(body, _dir));
    this.offset.copy(this.desiredOffset(0));
    this.update(body, 0);
  }

  update(body, dt) {
    const speed = body.speed;
    const steer = speed > 3 ? _dir.copy(body.velocity).setY(0).normalize() : flatForward(body, _dir);
    if (steer.lengthSq() > 0) this.heading.lerp(steer, 1 - Math.exp(-HEADING_RATE * dt)).normalize();
    this.offset.lerp(this.desiredOffset(speed), 1 - Math.exp(-FOLLOW_RATE * dt));
    const cam = this.camera.position.copy(body.position).add(this.offset);
    cam.y = Math.max(cam.y, this.terrain.heightAt(cam.x, cam.z) + MIN_CLEARANCE);
    this.camera.lookAt(_target.copy(body.position).addScaledVector(this.heading, 4).setY(body.position.y + 1.2));
    this.camera.fov = BASE_FOV + MAX_EXTRA_FOV * Math.min(speed / FOV_FULL_SPEED, 1);
    this.camera.updateProjectionMatrix();
  }

  desiredOffset(speed) {
    const distance = Math.min(BASE_DISTANCE + speed * DISTANCE_PER_SPEED, MAX_DISTANCE);
    return _desired.copy(this.heading).multiplyScalar(-distance).setY(BASE_HEIGHT + speed * HEIGHT_PER_SPEED);
  }
}

function flatForward(body, out) {
  return out.copy(body.forward).setY(0).normalize();
}
