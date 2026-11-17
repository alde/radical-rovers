import { comboMultiplier, isComboTrick } from './scoring.js';

// Time allowed after a landed trick before the chain banks. Hops and
// trickless jumps count against it once they land (at boost speed the rover
// is airborne most of the time, so pausing for every hop would mean a chain
// never ends); a flight that lands a trick resets it.
export const COMBO_WINDOW = 2.5; // seconds

/**
 * Points from consecutive landed jumps, held until the chain ends. Banking
 * pays them out with the chain multiplier; a bail drops the lot.
 */
export class ComboChain {
  constructor() {
    this.clear();
  }

  clear() {
    this.points = 0;
    this.trickCount = 0;
    this.jumps = 0;
    this.groundTime = 0;
  }

  get active() { return this.jumps > 0; }
  get multiplier() { return comboMultiplier(this.trickCount); }
  get value() { return Math.round(this.points * this.multiplier); }
  /** 1 just after a landing, 0 when the chain is about to bank. */
  get timeLeft() { return this.active ? 1 - this.groundTime / COMBO_WINDOW : 0; }

  /** Add a landed jump; its points are already multiplied by the landing grade. */
  add(tricks, jumpTotal, airtime) {
    const comboTricks = tricks.filter(isComboTrick).length;
    this.points += jumpTotal;
    this.trickCount += comboTricks;
    this.jumps++;
    if (comboTricks) this.groundTime = 0; // only real tricks buy more time
    else this.tick(airtime);
  }

  /** Advance the window; returns true once it has run out. */
  tick(dt) {
    if (!this.active) return false;
    this.groundTime += dt;
    return this.groundTime >= COMBO_WINDOW;
  }

  /** End the chain, returning what it was worth. */
  take() {
    const summary = { value: this.value, multiplier: this.multiplier, trickCount: this.trickCount };
    this.clear();
    return summary;
  }
}
