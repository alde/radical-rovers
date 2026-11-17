//! Follows the rover through each flight, scores it on touchdown and chains
//! consecutive landings into combos. Feed it one sample per frame.

use bevy::math::DVec3;

use super::combo::{ComboChain, ComboSummary};
use super::scoring::{Flight, Grade, Touchdown, Trick, base_points, describe_tricks, grade_landing};

/// Hops shorter than this are suspension chatter, not jumps. At boost speed
/// the rover is airborne most of the time, so this filters a lot.
const MIN_AIRTIME: f64 = 0.4; // seconds

/// Rover state reduced to plain numbers.
#[derive(Clone, Copy, Debug)]
pub struct FlightSample {
    pub airborne: bool,
    /// Body-frame angular velocity: x pitch, y yaw, z roll.
    pub local_spin: DVec3,
    pub height_above_ground: f64,
    pub alignment_deg: f64,
    pub slip_deg: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TrickEvent {
    Air { tricks: Vec<Trick>, base: u32 },
    Landed { tricks: Vec<Trick>, grade: Grade, base: u32, total: u32 },
    Bailed { tricks: Vec<Trick>, lost: u32 },
    Banked(ComboSummary),
}

#[derive(Default)]
struct FlightProgress {
    flight: Flight,
    spin_rate: f64,
}

#[derive(Default)]
pub struct TrickTracker {
    pub score: u32,
    pub combo: ComboChain,
    flight: Option<FlightProgress>,
}

impl TrickTracker {
    /// Teleport or respawn. On the ground the combo is banked; mid-air it is
    /// dropped, or resetting would dodge every bail.
    pub fn reset(&mut self) {
        if self.flight.take().is_some() {
            self.combo.clear();
        } else {
            self.score += self.combo.take().value;
        }
    }

    pub fn update(&mut self, dt: f64, sample: &FlightSample) -> Option<TrickEvent> {
        if sample.airborne {
            return self.fly(dt, sample);
        }
        if let Some(progress) = self.flight.take() {
            return self.land(progress, sample);
        }
        self.combo.tick(dt).then(|| self.bank())
    }

    fn fly(&mut self, dt: f64, sample: &FlightSample) -> Option<TrickEvent> {
        let p = self.flight.get_or_insert_with(FlightProgress::default);
        let spin = sample.local_spin;
        p.flight.pitch += spin.x * dt;
        p.flight.yaw += spin.y * dt;
        p.flight.roll += spin.z * dt;
        p.flight.airtime += dt;
        p.flight.max_height = p.flight.max_height.max(sample.height_above_ground);
        p.spin_rate = spin.length(); // judged from the air, before impact
        let tricks = describe_tricks(&p.flight);
        (!tricks.is_empty()).then(|| TrickEvent::Air { base: base_points(&tricks), tricks })
    }

    fn land(&mut self, progress: FlightProgress, sample: &FlightSample) -> Option<TrickEvent> {
        let flight = progress.flight;
        if flight.airtime < MIN_AIRTIME {
            return self.count_as_ground_time(flight.airtime);
        }
        let tricks = describe_tricks(&flight);
        let touchdown = Touchdown { alignment_deg: sample.alignment_deg, slip_deg: sample.slip_deg, spin_rate: progress.spin_rate };
        let grade = grade_landing(&touchdown);
        if grade == Grade::Bail {
            return self.bail(tricks);
        }
        if tricks.is_empty() {
            return self.count_as_ground_time(flight.airtime);
        }
        let base = base_points(&tricks);
        let total = base * grade.multiplier();
        self.combo.add(&tricks, total, flight.airtime);
        Some(TrickEvent::Landed { tricks, grade, base, total })
    }

    // Hops and trickless jumps run down the combo window; any banking happens
    // on the next ground frame.
    fn count_as_ground_time(&mut self, airtime: f64) -> Option<TrickEvent> {
        self.combo.tick(airtime);
        None
    }

    // A crash ends the chain even if this jump had no tricks of its own.
    fn bail(&mut self, tricks: Vec<Trick>) -> Option<TrickEvent> {
        if tricks.is_empty() && !self.combo.active() {
            return None;
        }
        let lost = self.combo.take().value + base_points(&tricks);
        Some(TrickEvent::Bailed { tricks, lost })
    }

    fn bank(&mut self) -> TrickEvent {
        let combo = self.combo.take();
        self.score += combo.value;
        TrickEvent::Banked(combo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    const DT: f64 = 1.0 / 60.0;

    fn air(spin: DVec3) -> FlightSample {
        FlightSample { airborne: true, local_spin: spin, height_above_ground: 5.0, alignment_deg: 0.0, slip_deg: 0.0 }
    }
    fn ground(alignment_deg: f64) -> FlightSample {
        FlightSample { airborne: false, local_spin: DVec3::ZERO, height_above_ground: 0.0, alignment_deg, slip_deg: 0.0 }
    }
    fn frames(seconds: f64) -> usize {
        (seconds / DT).round() as usize
    }

    /// Fly with a constant spin; returns the last event.
    fn fly(t: &mut TrickTracker, seconds: f64, spin: DVec3) -> Option<TrickEvent> {
        (0..frames(seconds)).map(|_| t.update(DT, &air(spin))).last().flatten()
    }

    /// Sit on the ground; returns the first event.
    fn wait(t: &mut TrickTracker, seconds: f64) -> Option<TrickEvent> {
        (0..frames(seconds)).filter_map(|_| t.update(DT, &ground(0.0))).next()
    }

    /// One full rotation about `axis` at one turn per second, stop, touch down.
    fn jump(t: &mut TrickTracker, axis: DVec3, alignment_deg: f64) -> Option<TrickEvent> {
        fly(t, 1.05, axis * TAU);
        fly(t, 0.2, DVec3::ZERO);
        t.update(DT, &ground(alignment_deg))
    }

    fn banked(event: Option<TrickEvent>) -> ComboSummary {
        match event {
            Some(TrickEvent::Banked(summary)) => summary,
            other => panic!("expected a bank, got {other:?}"),
        }
    }

    #[test]
    fn reports_tricks_in_progress_while_airborne() {
        let mut t = TrickTracker::default();
        let Some(TrickEvent::Air { tricks, .. }) = fly(&mut t, 1.05, DVec3::X * TAU) else { panic!() };
        assert_eq!(tricks[0].name, "Backflip");
    }

    #[test]
    fn a_perfectly_landed_backflip_is_worth_triple() {
        let event = jump(&mut TrickTracker::default(), DVec3::X, 3.0);
        let Some(TrickEvent::Landed { grade, base, total, .. }) = event else { panic!("{event:?}") };
        assert_eq!((grade, base, total), (Grade::Perfect, 1000, 3000));
    }

    #[test]
    fn a_combo_banks_into_the_score_once_the_rover_stays_down() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        assert_eq!(t.score, 0);
        assert_eq!(banked(wait(&mut t, 3.0)), ComboSummary { value: 3000, multiplier: 1.0, trick_count: 1 });
        assert_eq!(t.score, 3000);
    }

    #[test]
    fn chained_jumps_multiply_by_one_and_a_half_per_trick() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        wait(&mut t, 1.0);
        jump(&mut t, DVec3::Z, 0.0);
        wait(&mut t, 1.0);
        jump(&mut t, DVec3::X, 0.0);
        let combo = banked(wait(&mut t, 3.0));
        assert_eq!((combo.trick_count, combo.multiplier), (3, 2.25));
        assert_eq!(t.score, ((3000.0 + 2400.0 + 3000.0) * 2.25) as u32);
    }

    #[test]
    fn separate_jumps_outside_the_window_score_on_their_own() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        wait(&mut t, 3.0);
        jump(&mut t, DVec3::X, 0.0);
        wait(&mut t, 3.0);
        assert_eq!(t.score, 6000);
    }

    #[test]
    fn bumps_and_trickless_jumps_run_down_the_combo_window() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        for _ in 0..4 {
            fly(&mut t, 0.3, DVec3::ZERO); // bouncing over rough ground
            t.update(DT, &ground(0.0));
        }
        fly(&mut t, 1.0, DVec3::ZERO); // a jump with nothing in it
        t.update(DT, &ground(0.0));
        banked(wait(&mut t, 0.3));
    }

    #[test]
    fn a_trick_started_late_in_the_window_still_joins_the_chain() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        wait(&mut t, 2.3);
        jump(&mut t, DVec3::Z, 0.0);
        assert_eq!(banked(wait(&mut t, 3.0)).trick_count, 2);
    }

    #[test]
    fn bonus_only_jumps_add_points_but_neither_multiply_nor_extend() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        wait(&mut t, 1.5);
        fly(&mut t, 2.6, DVec3::ZERO); // Big Air, no rotation
        let Some(TrickEvent::Landed { total: big_air, .. }) = t.update(DT, &ground(0.0)) else { panic!() };
        let combo = banked(wait(&mut t, 1.1));
        assert_eq!((combo.multiplier, combo.value), (1.0, 3000 + big_air));
    }

    #[test]
    fn bailing_loses_the_whole_chain() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        wait(&mut t, 1.0);
        let event = jump(&mut t, DVec3::Z, 120.0);
        let Some(TrickEvent::Bailed { lost, .. }) = event else { panic!("{event:?}") };
        assert_eq!(lost, 3000 + 800); // banked-so-far plus this jump's raw points
        wait(&mut t, 3.0);
        assert_eq!(t.score, 0);
    }

    #[test]
    fn a_trickless_crash_still_breaks_the_chain() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        fly(&mut t, 1.0, DVec3::ZERO);
        assert!(matches!(t.update(DT, &ground(90.0)), Some(TrickEvent::Bailed { lost: 3000, .. })));
    }

    #[test]
    fn landing_mid_rotation_bails_even_if_the_angle_lines_up() {
        let mut t = TrickTracker::default();
        fly(&mut t, 3.0, DVec3::Z * TAU);
        assert!(matches!(t.update(DT, &ground(0.0)), Some(TrickEvent::Bailed { .. })));
    }

    #[test]
    fn short_hops_are_ignored_even_if_they_rotate() {
        let mut t = TrickTracker::default();
        fly(&mut t, 0.2, DVec3::X * 40.0);
        assert_eq!(t.update(DT, &ground(0.0)), None);
    }

    #[test]
    fn respawning_on_the_ground_banks_the_combo() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        t.reset();
        assert_eq!(t.score, 3000);
    }

    #[test]
    fn respawning_mid_air_drops_the_combo_and_the_flight() {
        let mut t = TrickTracker::default();
        jump(&mut t, DVec3::X, 0.0);
        fly(&mut t, 1.05, DVec3::X * TAU);
        t.reset();
        assert_eq!(t.update(DT, &ground(0.0)), None);
        assert_eq!(t.score, 0);
    }

    #[test]
    fn staying_on_the_ground_produces_no_events() {
        assert_eq!(wait(&mut TrickTracker::default(), 5.0), None);
    }
}
