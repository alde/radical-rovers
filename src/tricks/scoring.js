// Trick scoring rules. Pure functions, no Three.js, so they test in isolation.

const FULL_TURN = 2 * Math.PI;
const HALF_TURN = Math.PI;
// A rotation counts if you get within this much of completing it; the
// landing grade decides whether you actually got away with it.
const ROTATION_TOLERANCE = 0.6; // radians, ~35 degrees

const POINTS = {
  flip: 1000, // per full pitch rotation
  roll: 800, // per full barrel roll
  spin: 250, // per 180 of yaw
  airtimePerSecond: 100,
  heightPerMetre: 10,
};
const BIG_AIR_SECONDS = 2;
// Each rotation trick in a combo chain multiplies the chain by this.
const COMBO_STEP = 1.5;
const HIGH_FLYER_METRES = 25;

// Landing grades, best first. Alignment is the angle between the rover's up
// and the ground normal; slip is the angle between heading and travel.
const GRADES = [
  { id: 'perfect', label: 'PERFECT LANDING', maxAlignment: 12, maxSlip: 15, multiplier: 3 },
  { id: 'clean', label: 'CLEAN LANDING', maxAlignment: 25, maxSlip: 35, multiplier: 2 },
  { id: 'sketchy', label: 'SKETCHY', maxAlignment: 50, maxSlip: 180, multiplier: 1 },
];
const BAIL = { id: 'bail', label: 'BAIL', multiplier: 0 };
// Still rotating faster than this at touchdown and you didn't land it, you
// crashed into it. Stops "hold roll for the whole jump" farming points.
const MAX_LANDING_SPIN = 3; // rad/s, about half a turn per second

const PREFIXES = ['', '', 'Double ', 'Triple ', 'Quad '];
const repeated = (count, name) => `${PREFIXES[count] ?? `${count}× `}${name}`;

export function countRotations(radians, unit = FULL_TURN) {
  return Math.floor((Math.abs(radians) + ROTATION_TOLERANCE) / unit);
}

/**
 * Tricks earned by a flight. Rotations are accumulated body-frame angles in
 * radians: pitch < 0 is nose-down (front flip), yaw/roll either way.
 * Rotation tricks build combo chains; bonus tricks (hang time, height) only
 * add points, or boosting over bumps would chain forever.
 */
export function describeTricks({ pitch, roll, yaw, airtime, maxHeight }) {
  const tricks = [];
  const add = (kind, name, points) => tricks.push({ kind, name, points });
  const flips = countRotations(pitch);
  if (flips) add('rotation', repeated(flips, pitch < 0 ? 'Frontflip' : 'Backflip'), flips * POINTS.flip);
  const rolls = countRotations(roll);
  if (rolls) add('rotation', repeated(rolls, 'Barrel Roll'), rolls * POINTS.roll);
  const halfSpins = countRotations(yaw, HALF_TURN);
  if (halfSpins) add('rotation', `${halfSpins * 180}`, halfSpins * POINTS.spin);
  if (airtime >= BIG_AIR_SECONDS) add('bonus', `Big Air ${airtime.toFixed(1)}s`, Math.round(airtime * POINTS.airtimePerSecond));
  if (maxHeight >= HIGH_FLYER_METRES) add('bonus', `High Flyer ${Math.round(maxHeight)} m`, Math.round(maxHeight * POINTS.heightPerMetre));
  return tricks;
}

export function gradeLanding({ alignmentDeg, slipDeg, spinRate = 0 }) {
  if (spinRate > MAX_LANDING_SPIN) return BAIL;
  return GRADES.find((g) => alignmentDeg <= g.maxAlignment && slipDeg <= g.maxSlip) ?? BAIL;
}

export function scoreJump(tricks, grade) {
  const base = tricks.reduce((sum, t) => sum + t.points, 0);
  return { base, total: base * grade.multiplier };
}

export const isComboTrick = (trick) => trick.kind === 'rotation';

/** Chain multiplier: 1 trick ×1, 2 ×1.5, 3 ×2.25, ... */
export function comboMultiplier(trickCount) {
  return COMBO_STEP ** Math.max(0, trickCount - 1);
}
