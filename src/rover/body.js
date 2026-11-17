import * as THREE from 'three';
import { clamp } from '../util/math.js';

const UP = new THREE.Vector3(0, 1, 0);
const MIN_WHEEL_ALIGNMENT = 0.2; // below this |up . normal| the wheels can't touch (rolled over)
const FULL_GRIP_COMPRESSION = 0.005; // metres; grip ramps in over the first few mm. Low g means
// static sag is only ~8 mm (Moon) to ~19 mm (Mars), so this must stay well below that.
const HANDBRAKE_RATE = 4; // 1/s
const REVERSE_THRESHOLD = 1; // m/s; above this, "back" brakes instead of reversing
const HULL_FRICTION_RATE = 6; // 1/s cap on how fast hull scraping can stop you
const _levelAxis = new THREE.Vector3(), _groundNormal = new THREE.Vector3(), _travel = new THREE.Vector3(), _facing = new THREE.Vector3();
const MIN_WEATHERVANE_SPEED_SQ = 25; // (5 m/s)^2

const _mount = new THREE.Vector3(), _down = new THREE.Vector3(), _n = new THREE.Vector3();
const _contact = new THREE.Vector3(), _pv = new THREE.Vector3(), _f = new THREE.Vector3();
const _fwd = new THREE.Vector3(), _side = new THREE.Vector3(), _r = new THREE.Vector3();
const _p = new THREE.Vector3(), _t = new THREE.Vector3(), _acc = new THREE.Vector3();
const _tyrePoint = new THREE.Vector3();
const _qInv = new THREE.Quaternion(), _dq = new THREE.Quaternion();

/**
 * Rigid-body rover over a heightfield: four spring-damper wheels with arcade
 * tyre forces, penalty contacts on the hull corners, and air control.
 */
export class RoverBody {
  constructor(spec) {
    this.spec = spec;
    this.position = new THREE.Vector3();
    this.velocity = new THREE.Vector3();
    this.quaternion = new THREE.Quaternion();
    this.angularVelocity = new THREE.Vector3();
    this.right = new THREE.Vector3(); this.up = new THREE.Vector3(); this.forward = new THREE.Vector3();
    this.force = new THREE.Vector3(); this.torque = new THREE.Vector3(); this.airAccel = new THREE.Vector3();
    this.invInertia = boxInverseInertia(spec.mass, spec.halfExtents);
    this.corners = boxCorners(spec.halfExtents);
    this.wheels = spec.wheelMounts.map((mount) => ({ mount, steer: 0, extension: spec.restLength, groundSpeed: 0, grounded: false }));
    this.groundedWheels = 0;
    this.hullContacts = 0;
    this.updateBasis();
  }

  get speed() { return this.velocity.length(); }
  get forwardSpeed() { return this.velocity.dot(this.forward); }
  get heading() { return Math.atan2(-this.forward.x, -this.forward.z); }
  /** Touching the ground with anything: wheels or hull. */
  get inContact() { return this.groundedWheels > 0 || this.hullContacts > 0; }

  placeAt(x, y, z, heading = this.heading) {
    this.position.set(x, y, z);
    this.quaternion.setFromAxisAngle(UP, heading);
    this.velocity.set(0, 0, 0);
    this.angularVelocity.set(0, 0, 0);
    this.updateBasis();
  }

  step(dt, input, terrain, gravity) {
    this.force.set(0, -gravity * this.spec.mass, 0);
    this.torque.set(0, 0, 0);
    this.airAccel.set(0, 0, 0);
    this.groundedWheels = 0;
    this.hullContacts = 0;
    this.wheels.forEach((wheel) => this.applyWheel(wheel, input, terrain));
    this.corners.forEach((corner) => this.applyHullContact(corner, terrain));
    if (this.groundedWheels === 0) this.applyAirControl(input, terrain);
    if (this.groundedWheels === 0 && input.boost && input.throttle > 0) this.applyRocket();
    this.integrate(dt);
  }

  applyWheel(wheel, input, terrain) {
    const s = this.spec, reach = s.restLength + s.wheelRadius;
    const mount = _mount.copy(wheel.mount).applyQuaternion(this.quaternion).add(this.position);
    const n = terrain.normalAt(mount.x, mount.z, _n);
    const down = _down.copy(this.up).negate();
    const alignment = down.dot(n);
    wheel.grounded = false;
    wheel.extension = s.restLength;
    if (alignment > -MIN_WHEEL_ALIGNMENT) return;
    // Ray along the suspension axis against the local ground plane.
    const t = ((terrain.heightAt(mount.x, mount.z) - mount.y) * n.y) / alignment;
    wheel.extension = clamp(t - s.wheelRadius, 0, s.restLength);
    if (t >= reach) return;
    const compression = Math.min(reach - t, reach);
    const contact = _contact.copy(mount).addScaledVector(down, t);
    const pv = this.pointVelocity(contact, _pv);
    const spring = Math.max(0, s.springK * compression - s.springDamping * pv.dot(n));
    this.addForceAt(_f.copy(n).multiplyScalar(spring), contact);
    this.applyTyre(wheel, n, pv, compression, input);
    wheel.grounded = true;
    this.groundedWheels++;
  }

  // Tyre forces act at centre-of-mass height above the wheel, not at the
  // contact patch: an arcade trick so cornering yaws the rover but can't lever
  // it over. Slopes, kerbs and bad landings can still roll it.
  applyTyre(wheel, n, pv, compression, input) {
    const s = this.spec;
    wheel.steer = this.steerAngle(wheel, input);
    const fwd = _fwd.copy(this.forward).applyAxisAngle(this.up, wheel.steer);
    fwd.addScaledVector(n, -fwd.dot(n)).normalize();
    const side = _side.crossVectors(fwd, n);
    const vLong = pv.dot(fwd), vLat = pv.dot(side);
    const load = Math.min(1, compression / FULL_GRIP_COMPRESSION) * (s.mass / this.wheels.length);
    const lateral = clamp(-vLat * s.lateralGrip, -s.maxLateralAccel, s.maxLateralAccel) * load;
    const longitudinal = this.longitudinalAccel(vLong, input) * load;
    const at = _tyrePoint.set(wheel.mount.x, 0, wheel.mount.z).applyQuaternion(this.quaternion).add(this.position);
    this.addForceAt(_f.copy(side).multiplyScalar(lateral).addScaledVector(fwd, longitudinal), at);
    wheel.groundSpeed = vLong;
  }

  steerAngle(wheel, input) {
    const s = this.spec;
    const lock = s.maxSteer / (1 + Math.abs(this.forwardSpeed) / s.steerFalloffSpeed);
    const isFront = wheel.mount.z < 0;
    return -input.steer * lock * (isFront ? 1 : -s.rearSteerRatio);
  }

  longitudinalAccel(vLong, input) {
    const s = this.spec, gear = input.boost ? s.boost : s.cruise;
    if (input.handbrake) return -clamp(vLong * HANDBRAKE_RATE, -s.brakeDecel, s.brakeDecel);
    if (input.throttle > 0) return input.throttle * gear.accel * clamp(1 - vLong / gear.topSpeed, 0, 1);
    if (input.throttle < 0 && vLong > REVERSE_THRESHOLD) return input.throttle * s.brakeDecel;
    if (input.throttle < 0) return input.throttle * s.reverse.accel * clamp(1 + vLong / s.reverse.topSpeed, 0, 1);
    return -vLong * s.rollingResistance;
  }

  applyHullContact(corner, terrain) {
    const s = this.spec;
    const p = _p.copy(corner).applyQuaternion(this.quaternion).add(this.position);
    const ground = terrain.heightAt(p.x, p.z);
    if (p.y >= ground) return;
    this.hullContacts++;
    const n = terrain.normalAt(p.x, p.z, _n);
    const pv = this.pointVelocity(p, _pv);
    const vn = pv.dot(n);
    const normalForce = Math.max(0, s.hullK * (ground - p.y) * n.y - s.hullDamping * vn);
    const tangential = _t.copy(pv).addScaledVector(n, -vn), vt = tangential.length();
    const friction = vt > 1e-3 ? Math.min(s.hullFriction * normalForce, vt * s.mass * HULL_FRICTION_RATE) / vt : 0;
    this.addForceAt(_f.copy(n).multiplyScalar(normalForce).addScaledVector(tangential, -friction), p);
  }

  // Boost keeps pushing in the air; at speed you spend most of your time there.
  applyRocket() {
    const s = this.spec;
    const headroom = clamp(1 - this.forwardSpeed / s.boost.topSpeed, 0, 1);
    this.force.addScaledVector(this.forward, s.airThrust * s.mass * headroom);
  }

  applyAirControl(input, terrain) {
    const s = this.spec;
    this.airAccel.set(-input.pitch * s.airPitchAccel, -input.yaw * s.airYawAccel, -input.roll * s.airRollAccel);
    this.airAccel.applyQuaternion(this.quaternion);
    if (!input.pitch && !input.roll && !input.yaw) this.applyAirAssist(terrain);
  }

  // Hands off in the air, a gyro lines the rover up with the ground beneath it
  // and turns its nose into the direction of travel, so long jumps land on the
  // wheels and match the slope they come down on.
  applyAirAssist(terrain) {
    const s = this.spec;
    const ground = terrain.normalAt(this.position.x, this.position.z, _groundNormal);
    const tilt = _levelAxis.crossVectors(this.up, ground);
    this.airAccel.addScaledVector(tilt, s.airLevelStiffness).addScaledVector(this.angularVelocity, -s.airLevelDamping);
    const travel = _travel.copy(this.velocity).setY(0);
    if (travel.lengthSq() < MIN_WEATHERVANE_SPEED_SQ) return;
    const facing = _facing.copy(this.forward).setY(0).normalize();
    travel.normalize();
    if (facing.dot(travel) > 0) this.airAccel.y += facing.cross(travel).y * s.airWeathervane;
  }

  integrate(dt) {
    this.velocity.addScaledVector(this.force, dt / this.spec.mass);
    this.position.addScaledVector(this.velocity, dt);
    const angularAccel = this.worldInverseInertia(this.torque, _acc).add(this.airAccel);
    this.angularVelocity.addScaledVector(angularAccel, dt);
    this.angularVelocity.multiplyScalar(Math.max(0, 1 - this.spec.angularDamping * dt));
    const w = this.angularVelocity;
    _dq.set(w.x * dt * 0.5, w.y * dt * 0.5, w.z * dt * 0.5, 0).multiply(this.quaternion);
    const q = this.quaternion;
    q.set(q.x + _dq.x, q.y + _dq.y, q.z + _dq.z, q.w + _dq.w).normalize();
    this.updateBasis();
  }

  addForceAt(force, point) {
    this.force.add(force);
    this.torque.add(_r.subVectors(point, this.position).cross(force));
  }

  pointVelocity(point, out) {
    out.subVectors(point, this.position);
    return out.crossVectors(this.angularVelocity, out).add(this.velocity);
  }

  worldInverseInertia(torque, out) {
    _qInv.copy(this.quaternion).invert();
    return out.copy(torque).applyQuaternion(_qInv).multiply(this.invInertia).applyQuaternion(this.quaternion);
  }

  updateBasis() {
    this.right.set(1, 0, 0).applyQuaternion(this.quaternion);
    this.up.set(0, 1, 0).applyQuaternion(this.quaternion);
    this.forward.set(0, 0, -1).applyQuaternion(this.quaternion);
  }
}

function boxInverseInertia(mass, { x, y, z }) {
  return new THREE.Vector3(3 / (mass * (y * y + z * z)), 3 / (mass * (x * x + z * z)), 3 / (mass * (x * x + y * y)));
}

function boxCorners({ x, y, z }) {
  const corners = [];
  for (const sx of [-1, 1]) for (const sy of [-1, 1]) for (const sz of [-1, 1]) corners.push(new THREE.Vector3(sx * x, sy * y, sz * z));
  return corners;
}
