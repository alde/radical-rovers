// Synthesised feedback tones; no audio assets. Browsers only allow audio
// after a user gesture, so the context is created on the first key press.

const NOTE_LENGTH = 0.12; // seconds
const NOTE_GAP = 0.09;
const VOLUME = 0.18;
const ARPEGGIOS = {
  perfect: [523.25, 659.25, 783.99, 1046.5], // C major, rising: C5 E5 G5 C6
  clean: [523.25, 659.25, 783.99],
  sketchy: [392.0, 440.0],
  bail: [110, 82.4], // low and falling
  trick: [880], // a tick each time a new trick registers mid-air
  bank: [783.99, 1046.5, 1318.5, 1568.0, 2093.0], // combo cashed in: G5 up to C7
};

export class TrickSounds {
  constructor() {
    this.context = null;
    addEventListener('keydown', () => this.unlock(), { once: true });
  }

  unlock() {
    this.context ??= new AudioContext();
  }

  play(id) {
    const notes = ARPEGGIOS[id];
    if (!this.context || !notes) return;
    const start = this.context.currentTime;
    notes.forEach((frequency, i) => this.tone(frequency, start + i * NOTE_GAP, id === 'bail' ? 'sawtooth' : 'triangle'));
  }

  tone(frequency, at, type) {
    const oscillator = new OscillatorNode(this.context, { type, frequency });
    const gain = new GainNode(this.context, { gain: 0 });
    gain.gain.linearRampToValueAtTime(VOLUME, at + 0.01);
    gain.gain.exponentialRampToValueAtTime(0.001, at + NOTE_LENGTH * 2);
    oscillator.connect(gain).connect(this.context.destination);
    oscillator.start(at);
    oscillator.stop(at + NOTE_LENGTH * 2);
  }
}
