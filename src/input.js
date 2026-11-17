const AXES = {
  throttle: { positive: ['KeyW'], negative: ['KeyS'] },
  steer: { positive: ['KeyD'], negative: ['KeyA'] },
  pitch: { positive: ['ArrowUp'], negative: ['ArrowDown'] }, // air only: nose down / up
  roll: { positive: ['ArrowRight'], negative: ['ArrowLeft'] }, // air only
  yaw: { positive: ['KeyE'], negative: ['KeyQ'] }, // air only
};

/** Keyboard state plus one-shot presses for toggles. */
export class Input {
  constructor(target = window) {
    this.held = new Set();
    this.pressed = new Set();
    target.addEventListener('keydown', (e) => {
      if (e.target instanceof HTMLSelectElement) return;
      if (!e.repeat) this.pressed.add(e.code);
      this.held.add(e.code);
      if (e.code === 'Space' || e.code.startsWith('Arrow')) e.preventDefault();
    });
    target.addEventListener('keyup', (e) => this.held.delete(e.code));
    target.addEventListener('blur', () => this.held.clear());
  }

  get state() {
    const axis = ({ positive, negative }) => this.any(positive) - this.any(negative);
    return {
      throttle: axis(AXES.throttle),
      steer: axis(AXES.steer),
      pitch: axis(AXES.pitch),
      roll: axis(AXES.roll),
      yaw: axis(AXES.yaw),
      boost: this.any(['ShiftLeft', 'ShiftRight']),
      handbrake: this.any(['Space']),
    };
  }

  any(codes) { return codes.some((c) => this.held.has(c)) ? 1 : 0; }

  /** True once per key press. */
  consume(code) { return this.pressed.delete(code); }
}
