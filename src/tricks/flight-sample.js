import * as THREE from 'three';

const MIN_SLIP_SPEED = 3; // m/s; below this, heading vs travel is noise
const RAD_TO_DEG = 180 / Math.PI;

const _spin = new THREE.Vector3(), _normal = new THREE.Vector3();
const _facing = new THREE.Vector3(), _travel = new THREE.Vector3();
const _qInv = new THREE.Quaternion();

/** Reduces rover state to the plain numbers TrickTracker scores. */
export function sampleFlight(body, terrain) {
  const { x, y, z } = body.position;
  terrain.normalAt(x, z, _normal);
  return {
    airborne: !body.inContact,
    localSpin: _spin.copy(body.angularVelocity).applyQuaternion(_qInv.copy(body.quaternion).invert()),
    heightAboveGround: y - terrain.heightAt(x, z),
    alignmentDeg: body.up.angleTo(_normal) * RAD_TO_DEG,
    slipDeg: slipAngle(body) * RAD_TO_DEG,
  };
}

function slipAngle(body) {
  _travel.copy(body.velocity).setY(0);
  if (_travel.length() < MIN_SLIP_SPEED) return 0;
  _facing.copy(body.forward).setY(0);
  return _facing.lengthSq() > 0 ? _facing.angleTo(_travel) : Math.PI;
}
