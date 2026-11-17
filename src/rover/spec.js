import * as THREE from 'three';

const v = (x, y, z) => new THREE.Vector3(x, y, z);

// Arcade-tuned rover. Body frame: +x right, +y up, -z forward. Units SI.
export const ROVER_SPEC = {
  mass: 900,
  halfExtents: v(1.0, 0.4, 1.5),
  wheelMounts: [v(-1.2, -0.15, -1.25), v(1.2, -0.15, -1.25), v(-1.2, -0.15, 1.25), v(1.2, -0.15, 1.25)],
  wheelRadius: 0.5,
  wheelWidth: 0.45,
  restLength: 0.7,
  springK: 45000, // ~2.2 Hz per corner
  springDamping: 3500, // ~0.5 damping ratio
  hullK: 300000,
  hullDamping: 15000,
  hullFriction: 0.5,
  maxSteer: 0.55,
  steerFalloffSpeed: 25, // m/s at which steering lock halves
  rearSteerRatio: 0.5, // rear wheels counter-steer, like real rover corner drives
  lateralGrip: 8, // 1/s: time constant for killing sideways slip
  maxLateralAccel: 30,
  cruise: { topSpeed: 70, accel: 16 },
  boost: { topSpeed: 420, accel: 80 },
  reverse: { topSpeed: 15, accel: 8 },
  airThrust: 30, // m/s^2 from the rockets while boosting airborne
  brakeDecel: 35,
  rollingResistance: 0.05,
  airPitchAccel: 5, // rad/s^2
  airRollAccel: 6,
  airYawAccel: 3,
  airLevelStiffness: 6, // rad/s^2 per radian-ish of tilt
  airLevelDamping: 3, // 1/s
  airWeathervane: 8, // rad/s^2 per unit sin(yaw error)
  angularDamping: 0.3,
};
