//! Points from consecutive landed jumps, held until the chain ends. Banking
//! pays them out with the chain multiplier; a bail drops the lot.

use super::scoring::{Trick, TrickKind, combo_multiplier};

/// Time allowed after a landed trick before the chain banks. Hops and
/// trickless jumps count against it once they land (at boost speed the rover
/// is airborne most of the time, so pausing for every hop would mean a chain
/// never ends); a flight that lands a trick resets it.
pub const COMBO_WINDOW: f64 = 2.5; // seconds

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ComboSummary {
    pub value: u32,
    pub multiplier: f64,
    pub trick_count: u32,
}

#[derive(Default)]
pub struct ComboChain {
    points: u32,
    trick_count: u32,
    jumps: u32,
    window_used: f64,
}

impl ComboChain {
    pub fn active(&self) -> bool {
        self.jumps > 0
    }

    pub fn multiplier(&self) -> f64 {
        combo_multiplier(self.trick_count)
    }

    pub fn value(&self) -> u32 {
        (f64::from(self.points) * self.multiplier()).round() as u32
    }

    /// 1 just after a landed trick, 0 when the chain is about to bank.
    pub fn time_left(&self) -> f64 {
        if self.active() { (1.0 - self.window_used / COMBO_WINDOW).max(0.0) } else { 0.0 }
    }

    /// Add a landed jump; `jump_total` is already multiplied by the landing grade.
    pub fn add(&mut self, tricks: &[Trick], jump_total: u32, airtime: f64) {
        let combo_tricks = tricks.iter().filter(|t| t.kind == TrickKind::Rotation).count() as u32;
        self.points += jump_total;
        self.trick_count += combo_tricks;
        self.jumps += 1;
        if combo_tricks > 0 {
            self.window_used = 0.0; // only real tricks buy more time
        } else {
            self.tick(airtime);
        }
    }

    /// Advance the window; true once it has run out.
    pub fn tick(&mut self, dt: f64) -> bool {
        if !self.active() {
            return false;
        }
        self.window_used += dt;
        self.window_used >= COMBO_WINDOW
    }

    /// End the chain, returning what it was worth.
    pub fn take(&mut self) -> ComboSummary {
        let summary = ComboSummary { value: self.value(), multiplier: self.multiplier(), trick_count: self.trick_count };
        *self = Self::default();
        summary
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
