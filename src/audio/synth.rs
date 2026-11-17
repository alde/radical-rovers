//! Tiny offline synthesiser: renders the menu loop and trick chimes to PCM at
//! startup, so the game ships with no audio assets.

use std::f64::consts::TAU;

pub const SAMPLE_RATE: u32 = 44_100;

const BPM: f64 = 100.0;
const STEPS_PER_BAR: usize = 16; // sixteenth notes
const BARS: usize = 8;
const PEAK: f32 = 0.8;

/// Am - F - C - G, twice. (bass root, chord tones) as MIDI notes.
const PROGRESSION: [(u8, [u8; 3]); 4] = [(45, [57, 60, 64]), (41, [53, 57, 60]), (48, [60, 64, 67]), (43, [55, 59, 62])];

fn midi_to_hz(note: f64) -> f64 {
    440.0 * 2f64.powf((note - 69.0) / 12.0)
}

fn step_seconds() -> f64 {
    60.0 / BPM / 4.0
}

pub fn loop_seconds() -> f64 {
    step_seconds() * (STEPS_PER_BAR * BARS) as f64
}

/// Deterministic white noise for drums.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f64::from(self.0) / f64::from(u32::MAX) * 2.0 - 1.0
    }
}

/// Adds one voice into `out` starting at sample `start`.
fn add_note(out: &mut [f64], start: usize, seconds: f64, gain: f64, mut voice: impl FnMut(f64) -> f64) {
    let length = (seconds * f64::from(SAMPLE_RATE)) as usize;
    for i in 0..length {
        let t = i as f64 / f64::from(SAMPLE_RATE);
        let index = (start + i) % out.len(); // wrap so the loop's tails are seamless
        out[index] += voice(t) * gain;
    }
}

fn saw(phase: f64) -> f64 {
    2.0 * (phase - (phase + 0.5).floor())
}

fn pulse(phase: f64) -> f64 {
    if phase.fract() < 0.35 { 1.0 } else { -1.0 }
}

/// The menu loop: saw bass, pulse arpeggio with echo, kick, snare and hats.
pub fn menu_loop() -> Vec<f32> {
    let total = (loop_seconds() * f64::from(SAMPLE_RATE)) as usize;
    let mut out = vec![0.0; total];
    let mut noise = Noise(0x2545_f491);
    let step = step_seconds();
    let at = |s: usize| (s as f64 * step * f64::from(SAMPLE_RATE)) as usize;
    for s in 0..STEPS_PER_BAR * BARS {
        let (root, chord) = PROGRESSION[(s / STEPS_PER_BAR) % PROGRESSION.len()];
        if s % 2 == 0 {
            let note = f64::from(root) + if s % 8 == 6 { 12.0 } else { 0.0 };
            let hz = midi_to_hz(note);
            add_note(&mut out, at(s), step * 1.8, 0.28, |t| saw(hz * t) * (-t * 6.0).exp());
        }
        let arp = [chord[0], chord[1], chord[2], chord[1] + 12];
        let hz = midi_to_hz(f64::from(arp[s % arp.len()]) + 12.0);
        add_note(&mut out, at(s), step * 0.9, 0.12, |t| pulse(hz * t) * (-t * 9.0).exp());
        if s % 4 == 0 {
            add_note(&mut out, at(s), 0.3, 0.7, |t| (TAU * (45.0 * t + 18.0 * (1.0 - (-t * 30.0).exp()))).sin() * (-t * 9.0).exp());
        }
        if s % 8 == 4 {
            add_note(&mut out, at(s), 0.25, 0.32, |t| noise.next() * (-t * 14.0).exp());
        } else if s % 2 == 1 {
            add_note(&mut out, at(s), 0.05, 0.08, |t| noise.next() * (-t * 60.0).exp());
        }
    }
    add_echo(&mut out, at(3), 0.35);
    normalise(&out)
}

/// Feedback delay, applied around the loop twice so the tail wraps cleanly.
fn add_echo(out: &mut [f64], delay: usize, feedback: f64) {
    let len = out.len();
    for _ in 0..2 {
        for i in 0..len {
            out[i] += out[(i + len - delay) % len] * feedback;
        }
    }
}

/// A rising (or falling) arpeggio of short notes, for trick feedback.
pub fn chime(notes_hz: &[f64], harsh: bool) -> Vec<f32> {
    const GAP: f64 = 0.09;
    const NOTE: f64 = 0.24;
    let total = ((GAP * notes_hz.len() as f64 + NOTE) * f64::from(SAMPLE_RATE)) as usize;
    let mut out = vec![0.0; total];
    for (i, &hz) in notes_hz.iter().enumerate() {
        let start = (i as f64 * GAP * f64::from(SAMPLE_RATE)) as usize;
        add_note(&mut out, start, NOTE, 0.5, |t| {
            let wave = if harsh { saw(hz * t) } else { (TAU * hz * t).sin() * 0.7 + (TAU * hz * 2.0 * t).sin() * 0.3 };
            wave * (-t * 12.0).exp()
        });
    }
    normalise(&out)
}

fn normalise(samples: &[f64]) -> Vec<f32> {
    let peak = samples.iter().fold(0.0f64, |m, s| m.max(s.abs())).max(1e-9);
    samples.iter().map(|s| (s / peak) as f32 * PEAK).collect()
}

/// 16-bit mono PCM WAV.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend(b"RIFF");
    out.extend((36 + data_len).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes()); // fmt chunk size
    out.extend(1u16.to_le_bytes()); // PCM
    out.extend(1u16.to_le_bytes()); // mono
    out.extend(SAMPLE_RATE.to_le_bytes());
    out.extend((SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    out.extend(2u16.to_le_bytes()); // block align
    out.extend(16u16.to_le_bytes()); // bits per sample
    out.extend(b"data");
    out.extend(data_len.to_le_bytes());
    for s in samples {
        out.extend(((s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loop_is_eight_bars_at_100_bpm() {
        assert!((loop_seconds() - 19.2).abs() < 1e-9);
        assert_eq!(menu_loop().len(), (19.2 * f64::from(SAMPLE_RATE)) as usize);
    }

    #[test]
    fn output_is_normalised_below_clipping() {
        let peak = chime(&[440.0, 660.0], false).iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - PEAK).abs() < 1e-6);
    }

    #[test]
    fn wav_header_describes_the_data() {
        let wav = wav_bytes(&[0.0, 0.5, -0.5]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(wav.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
    }

    #[test]
    fn a4_is_440_hz() {
        assert!((midi_to_hz(69.0) - 440.0).abs() < 1e-9);
    }
}
