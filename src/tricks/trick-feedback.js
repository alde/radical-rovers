import { TrickSounds } from './sounds.js';

const POPUP_SECONDS = 2.6;
const BANK_FLASH_SECONDS = 1.6;
const $ = (id) => document.getElementById(id);
const fmt = (n) => Math.round(n).toLocaleString('en-GB');
const formatMultiplier = (m) => `×${Number(m.toFixed(2))}`;
const trickLine = (tricks) => tricks.map((t) => t.name).join(' + ');

/**
 * Score counter, combo meter, live trick readout while airborne, landing
 * popups and sounds. Call handle() every frame with the tracker's event.
 */
export class TrickFeedback {
  constructor() {
    this.sounds = new TrickSounds();
    this.trickCount = 0;
    this.timers = {};
  }

  handle(event, tracker) {
    if (event?.type === 'air') this.showAir(event);
    else this.clearAir();
    if (event?.type === 'landed') this.showLanding(event);
    if (event?.type === 'banked') this.showBanked(event);
    this.renderScore(tracker.score);
    this.renderCombo(tracker.combo);
  }

  showAir({ tricks, base }) {
    if (tricks.length > this.trickCount) this.sounds.play('trick');
    this.trickCount = tricks.length;
    setText('trick-live', `${trickLine(tricks)}  ${fmt(base)}`);
    $('trick-live').classList.add('visible');
  }

  clearAir() {
    this.trickCount = 0;
    $('trick-live').classList.remove('visible');
  }

  showLanding({ tricks, grade, total, lost }) {
    const bailed = lost !== undefined;
    setText('popup-grade', bailed ? grade.label : `${grade.label} ${formatMultiplier(grade.multiplier)}`);
    setText('popup-tricks', trickLine(tricks));
    setText('popup-points', bailed ? `combo lost ${fmt(lost)}` : `+${fmt(total)}`);
    this.flash('trick-popup', `show ${grade.id}`, POPUP_SECONDS);
    this.sounds.play(grade.id);
  }

  showBanked({ value, multiplier, trickCount }) {
    if (!value) return;
    const chain = trickCount > 1 ? `${trickCount}-trick combo ${formatMultiplier(multiplier)}  ` : '';
    setText('bank-flash', `${chain}+${fmt(value)}`);
    this.flash('bank-flash', 'show', BANK_FLASH_SECONDS);
    this.sounds.play('bank');
  }

  renderScore(score) {
    setText('score', fmt(score));
  }

  renderCombo(combo) {
    $('combo').classList.toggle('active', combo.active);
    if (!combo.active) return;
    setText('combo-multiplier', formatMultiplier(combo.multiplier));
    setText('combo-value', fmt(combo.value));
    $('combo-timer').style.transform = `scaleX(${Math.max(0, combo.timeLeft)})`;
  }

  // Restart a CSS animation even if the previous one is still running.
  flash(id, className, seconds) {
    const el = $(id);
    el.className = '';
    void el.offsetWidth;
    el.className = className;
    clearTimeout(this.timers[id]);
    this.timers[id] = setTimeout(() => (el.className = ''), seconds * 1000);
  }
}

// Avoid touching the DOM when nothing changed; this runs every frame.
function setText(id, text) {
  const el = $(id);
  if (el.textContent !== text) el.textContent = text;
}
