import { describeTricks, gradeLanding, scoreJump } from './scoring.js';
import { ComboChain } from './combo-chain.js';

// Hops shorter than this are just suspension chatter, not jumps. At boost
// speed the rover is airborne most of the time, so this filters a lot.
const MIN_AIRTIME = 0.4; // seconds

/**
 * Follows the rover through each flight, scores it on touchdown and chains
 * consecutive landings into combos. Feed it one FlightSample per frame; it
 * returns an event or null:
 *   { type: 'air', tricks, base }                      airborne, tricks so far
 *   { type: 'landed', tricks, grade, base, total }     landed, jump added to combo
 *   { type: 'landed', tricks, grade, base, total: 0, lost }   bailed, combo lost
 *   { type: 'banked', value, multiplier, trickCount }  combo ended and paid out
 * Samples are plain numbers so this has no rendering dependencies.
 */
export class TrickTracker {
  constructor() {
    this.score = 0;
    this.flight = null;
    this.combo = new ComboChain();
  }

  /**
   * Teleport or respawn. On the ground the combo is banked; mid-air it is
   * dropped, or resetting would dodge every bail.
   */
  reset() {
    if (this.flight) this.combo.clear();
    else this.score += this.combo.take().value;
    this.flight = null;
  }

  update(dt, sample) {
    if (sample.airborne) return this.fly(dt, sample);
    if (this.flight) return this.land(sample);
    return this.combo.tick(dt) ? this.bank() : null;
  }

  fly(dt, { localSpin, heightAboveGround }) {
    const f = (this.flight ??= { pitch: 0, roll: 0, yaw: 0, airtime: 0, maxHeight: 0, spinRate: 0 });
    f.pitch += localSpin.x * dt;
    f.yaw += localSpin.y * dt;
    f.roll += localSpin.z * dt;
    f.airtime += dt;
    f.maxHeight = Math.max(f.maxHeight, heightAboveGround);
    f.spinRate = Math.hypot(localSpin.x, localSpin.y, localSpin.z); // judged from the air, before impact
    const tricks = describeTricks(f);
    return tricks.length ? { type: 'air', tricks, base: scoreJump(tricks, { multiplier: 1 }).base } : null;
  }

  land(sample) {
    const flight = this.flight;
    this.flight = null;
    if (flight.airtime < MIN_AIRTIME) return this.countAsGroundTime(flight);
    const tricks = describeTricks(flight);
    const grade = gradeLanding({ ...sample, spinRate: flight.spinRate });
    if (grade.multiplier === 0) return this.bail(tricks, grade);
    if (!tricks.length) return this.countAsGroundTime(flight);
    const { base, total } = scoreJump(tricks, grade);
    this.combo.add(tricks, total, flight.airtime);
    return { type: 'landed', tricks, grade, base, total };
  }

  // Hops and trickless jumps run down the combo window; any banking happens
  // on the next ground frame.
  countAsGroundTime(flight) {
    this.combo.tick(flight.airtime);
    return null;
  }

  // A crash ends the chain even if this jump had no tricks of its own.
  bail(tricks, grade) {
    if (!tricks.length && !this.combo.active) return null;
    const { base } = scoreJump(tricks, grade);
    const lost = this.combo.take().value + base;
    return { type: 'landed', tricks, grade, base, total: 0, lost };
  }

  bank() {
    const combo = this.combo.take();
    this.score += combo.value;
    return { type: 'banked', ...combo };
  }
}
