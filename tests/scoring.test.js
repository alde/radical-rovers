import { expect } from 'chai';
import { test } from './harness.js';
import { comboMultiplier, countRotations, describeTricks, gradeLanding, scoreJump } from '../src/tricks/scoring.js';

const TURN = 2 * Math.PI;
const deg = (d) => (d * Math.PI) / 180;
const flight = (overrides) => ({ pitch: 0, roll: 0, yaw: 0, airtime: 1, maxHeight: 5, ...overrides });
const names = (tricks) => tricks.map((t) => t.name);

test('a rotation 30 degrees short still counts', () => {
  expect(countRotations(TURN - deg(30))).to.equal(1);
});

test('a rotation 60 degrees short does not count', () => {
  expect(countRotations(TURN - deg(60))).to.equal(0);
});

test('nose-down pitch is a frontflip, nose-up a backflip', () => {
  expect(names(describeTricks(flight({ pitch: -TURN })))).to.deep.equal(['Frontflip']);
  expect(names(describeTricks(flight({ pitch: TURN })))).to.deep.equal(['Backflip']);
});

test('repeated rotations get a prefix and multiply points', () => {
  const [trick] = describeTricks(flight({ pitch: 2 * TURN }));
  expect(trick).to.deep.equal({ kind: 'rotation', name: 'Double Backflip', points: 2000 });
});

test('yaw is scored in 180s', () => {
  expect(names(describeTricks(flight({ yaw: deg(540) })))).to.deep.equal(['540']);
});

test('combined tricks are listed together', () => {
  const tricks = describeTricks(flight({ pitch: -TURN, roll: TURN, yaw: deg(360) }));
  expect(names(tricks)).to.deep.equal(['Frontflip', 'Barrel Roll', '360']);
});

test('long hang time and height earn their own tricks', () => {
  const tricks = describeTricks(flight({ airtime: 3.2, maxHeight: 40 }));
  expect(names(tricks)).to.deep.equal(['Big Air 3.2s', 'High Flyer 40 m']);
});

test('a short, flat hop earns nothing', () => {
  expect(describeTricks(flight({}))).to.deep.equal([]);
});

test('landing grades step down with alignment and slip', () => {
  expect(gradeLanding({ alignmentDeg: 5, slipDeg: 5 }).id).to.equal('perfect');
  expect(gradeLanding({ alignmentDeg: 20, slipDeg: 5 }).id).to.equal('clean');
  expect(gradeLanding({ alignmentDeg: 5, slipDeg: 30 }).id).to.equal('clean');
  expect(gradeLanding({ alignmentDeg: 40, slipDeg: 5 }).id).to.equal('sketchy');
  expect(gradeLanding({ alignmentDeg: 5, slipDeg: 120 }).id).to.equal('sketchy');
  expect(gradeLanding({ alignmentDeg: 80, slipDeg: 0 }).id).to.equal('bail');
});

test('the landing grade multiplies the trick points', () => {
  const tricks = [{ name: 'a', points: 1000 }, { name: 'b', points: 250 }];
  expect(scoreJump(tricks, gradeLanding({ alignmentDeg: 0, slipDeg: 0 }))).to.deep.equal({ base: 1250, total: 3750 });
  expect(scoreJump(tricks, gradeLanding({ alignmentDeg: 90, slipDeg: 0 })).total).to.equal(0);
});

test('touching down while still spinning is a bail, however level', () => {
  expect(gradeLanding({ alignmentDeg: 0, slipDeg: 0, spinRate: 5 }).id).to.equal('bail');
  expect(gradeLanding({ alignmentDeg: 0, slipDeg: 0, spinRate: 1 }).id).to.equal('perfect');
});

test('hang time and height are bonuses, rotations are combo tricks', () => {
  const tricks = describeTricks(flight({ roll: TURN, airtime: 3, maxHeight: 30 }));
  expect(tricks.map((t) => t.kind)).to.deep.equal(['rotation', 'bonus', 'bonus']);
});

test('the combo multiplier compounds by 1.5 per trick', () => {
  expect([1, 2, 3, 5].map(comboMultiplier)).to.deep.equal([1, 1.5, 2.25, 5.0625]);
});
