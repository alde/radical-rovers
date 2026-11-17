import { expect } from 'chai';
import { test } from './harness.js';
import { TrickTracker } from '../src/tricks/trick-tracker.js';

const DT = 1 / 60;
const TURN_PER_SECOND = 2 * Math.PI;
const air = (spin = {}, heightAboveGround = 5) => ({ airborne: true, localSpin: { x: 0, y: 0, z: 0, ...spin }, heightAboveGround });
const ground = (alignmentDeg = 0, slipDeg = 0) => ({ airborne: false, alignmentDeg, slipDeg });

// Fly for `seconds` with a constant body-frame spin; returns the last event.
function fly(tracker, seconds, spin) {
  let event = null;
  for (let t = 0; t < seconds; t += DT) event = tracker.update(DT, air(spin));
  return event;
}

// Sit on the ground for `seconds`; returns the first non-null event.
function wait(tracker, seconds) {
  let first = null;
  for (let t = 0; t < seconds; t += DT) first ??= tracker.update(DT, ground());
  return first;
}

// One full rotation about `axis`, stop rotating, then touch down.
function jump(tracker, axis = 'x', landing = ground()) {
  fly(tracker, 1.05, { [axis]: TURN_PER_SECOND });
  fly(tracker, 0.2, {});
  return tracker.update(DT, landing);
}

test('reports tricks in progress while airborne', () => {
  const tracker = new TrickTracker();
  const event = fly(tracker, 1.05, { x: TURN_PER_SECOND });
  expect(event.type).to.equal('air');
  expect(event.tricks[0].name).to.equal('Backflip');
});

test('a perfectly landed backflip is worth triple', () => {
  const event = jump(new TrickTracker(), 'x', ground(3, 3));
  expect(event).to.include({ type: 'landed', base: 1000, total: 3000 });
  expect(event.grade.id).to.equal('perfect');
});

test('a combo banks into the score once the rover stays down', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  expect(tracker.score).to.equal(0);
  const banked = wait(tracker, 3);
  expect(banked).to.include({ type: 'banked', value: 3000, multiplier: 1 });
  expect(tracker.score).to.equal(3000);
});

test('chained jumps multiply by 1.5 per trick', () => {
  const tracker = new TrickTracker();
  jump(tracker, 'x');
  wait(tracker, 1);
  jump(tracker, 'z');
  wait(tracker, 1);
  jump(tracker, 'x');
  const banked = wait(tracker, 3);
  expect(banked).to.include({ trickCount: 3, multiplier: 2.25 });
  expect(tracker.score).to.equal((3000 + 2400 + 3000) * 2.25);
});

test('separate jumps outside the window score on their own', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  wait(tracker, 3);
  jump(tracker);
  wait(tracker, 3);
  expect(tracker.score).to.equal(6000);
});

test('bumps and trickless jumps run down the combo window', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  for (let i = 0; i < 4; i++) {
    fly(tracker, 0.3, {}); // bouncing over rough ground
    tracker.update(DT, ground());
  }
  fly(tracker, 1, {}); // a jump with nothing in it
  tracker.update(DT, ground());
  expect(wait(tracker, 0.3)).to.include({ type: 'banked' });
});

test('a trick started late in the window still joins the chain', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  wait(tracker, 2.3);
  jump(tracker, 'z'); // airborne well past the window, but it lands a trick
  expect(wait(tracker, 3)).to.include({ type: 'banked', trickCount: 2 });
});

test('bonus-only jumps add points but neither multiply nor extend the chain', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  wait(tracker, 1.5);
  fly(tracker, 2.6, {}); // Big Air, no rotation
  const bigAir = tracker.update(DT, ground());
  expect(bigAir.tricks.map((t) => t.kind)).to.deep.equal(['bonus']);
  const banked = wait(tracker, 1.1);
  expect(banked).to.include({ type: 'banked', multiplier: 1, value: 3000 + bigAir.total });
});

test('bailing loses the whole chain', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  wait(tracker, 1);
  const event = jump(tracker, 'z', ground(120, 0));
  expect(event.grade.id).to.equal('bail');
  expect(event.lost).to.equal(3000 + 800); // banked-so-far plus this jump's raw points
  wait(tracker, 3);
  expect(tracker.score).to.equal(0);
});

test('a trickless crash still breaks the chain', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  fly(tracker, 1, {});
  const event = tracker.update(DT, ground(90, 0));
  expect(event).to.include({ type: 'landed', lost: 3000 });
});

test('landing mid-rotation bails even if the angle happens to line up', () => {
  const tracker = new TrickTracker();
  fly(tracker, 3, { z: TURN_PER_SECOND });
  expect(tracker.update(DT, ground(0, 0)).grade.id).to.equal('bail');
});

test('short hops are ignored even if they rotate', () => {
  const tracker = new TrickTracker();
  fly(tracker, 0.2, { x: 40 });
  expect(tracker.update(DT, ground())).to.equal(null);
});

test('respawning on the ground banks the combo', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  tracker.reset();
  expect(tracker.score).to.equal(3000);
});

test('respawning mid-air drops the combo and the flight', () => {
  const tracker = new TrickTracker();
  jump(tracker);
  fly(tracker, 1.05, { x: TURN_PER_SECOND });
  tracker.reset();
  expect(tracker.update(DT, ground())).to.equal(null);
  expect(tracker.score).to.equal(0);
});

test('staying on the ground produces no events', () => {
  expect(wait(new TrickTracker(), 5)).to.equal(null);
});
