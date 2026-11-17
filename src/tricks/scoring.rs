//! Trick scoring rules. Pure functions, so they test in isolation.

use std::f64::consts::{PI, TAU};

/// A rotation counts if you get within this much of completing it; the
/// landing grade decides whether you actually got away with it.
const ROTATION_TOLERANCE: f64 = 0.6; // radians, ~35 degrees

const POINTS_PER_FLIP: u32 = 1000;
const POINTS_PER_ROLL: u32 = 800;
const POINTS_PER_HALF_SPIN: u32 = 250;
const POINTS_PER_AIR_SECOND: f64 = 100.0;
const POINTS_PER_METRE: f64 = 10.0;
const BIG_AIR_SECONDS: f64 = 2.0;
const HIGH_FLYER_METRES: f64 = 25.0;
/// Each rotation trick in a combo chain multiplies the chain by this.
const COMBO_STEP: f64 = 1.5;
/// Still rotating faster than this at touchdown and you didn't land it, you
/// crashed into it. Stops "hold roll for the whole jump" farming points.
const MAX_LANDING_SPIN: f64 = 3.0; // rad/s, about half a turn per second

/// Rotation tricks build combo chains; bonuses (hang time, height) only add
/// points, or boosting over bumps would chain forever.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrickKind {
    Rotation,
    Bonus,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Trick {
    pub kind: TrickKind,
    pub name: String,
    pub points: u32,
}

/// Accumulated body-frame rotation (radians) and stats for one flight.
/// Pitch < 0 is nose-down (front flip).
#[derive(Clone, Copy, Default, Debug)]
pub struct Flight {
    pub pitch: f64,
    pub roll: f64,
    pub yaw: f64,
    pub airtime: f64,
    pub max_height: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grade {
    Perfect,
    Clean,
    Sketchy,
    Bail,
}

impl Grade {
    pub fn multiplier(self) -> u32 {
        match self {
            Grade::Perfect => 3,
            Grade::Clean => 2,
            Grade::Sketchy => 1,
            Grade::Bail => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Grade::Perfect => "PERFECT LANDING",
            Grade::Clean => "CLEAN LANDING",
            Grade::Sketchy => "SKETCHY",
            Grade::Bail => "BAIL",
        }
    }
}

/// How the rover met the ground. Alignment: angle between rover up and the
/// ground normal. Slip: angle between heading and travel. Spin: rad/s just
/// before touchdown.
#[derive(Clone, Copy, Default, Debug)]
pub struct Touchdown {
    pub alignment_deg: f64,
    pub slip_deg: f64,
    pub spin_rate: f64,
}

pub fn count_rotations(radians: f64, unit: f64) -> u32 {
    ((radians.abs() + ROTATION_TOLERANCE) / unit).floor() as u32
}

fn repeated(count: u32, name: &str) -> String {
    match count {
        1 => name.to_string(),
        2 => format!("Double {name}"),
        3 => format!("Triple {name}"),
        4 => format!("Quad {name}"),
        n => format!("{n}× {name}"),
    }
}

pub fn describe_tricks(f: &Flight) -> Vec<Trick> {
    let mut tricks = Vec::new();
    let mut add = |kind, name: String, points| tricks.push(Trick { kind, name, points });
    let flips = count_rotations(f.pitch, TAU);
    if flips > 0 {
        add(TrickKind::Rotation, repeated(flips, if f.pitch < 0.0 { "Frontflip" } else { "Backflip" }), flips * POINTS_PER_FLIP);
    }
    let rolls = count_rotations(f.roll, TAU);
    if rolls > 0 {
        add(TrickKind::Rotation, repeated(rolls, "Barrel Roll"), rolls * POINTS_PER_ROLL);
    }
    let half_spins = count_rotations(f.yaw, PI);
    if half_spins > 0 {
        add(TrickKind::Rotation, format!("{}", half_spins * 180), half_spins * POINTS_PER_HALF_SPIN);
    }
    if f.airtime >= BIG_AIR_SECONDS {
        add(TrickKind::Bonus, format!("Big Air {:.1}s", f.airtime), (f.airtime * POINTS_PER_AIR_SECOND).round() as u32);
    }
    if f.max_height >= HIGH_FLYER_METRES {
        add(TrickKind::Bonus, format!("High Flyer {} m", f.max_height.round()), (f.max_height * POINTS_PER_METRE).round() as u32);
    }
    tricks
}

const GRADE_LIMITS: [(Grade, f64, f64); 3] =
    [(Grade::Perfect, 12.0, 15.0), (Grade::Clean, 25.0, 35.0), (Grade::Sketchy, 50.0, 180.0)];

pub fn grade_landing(t: &Touchdown) -> Grade {
    if t.spin_rate > MAX_LANDING_SPIN {
        return Grade::Bail;
    }
    GRADE_LIMITS
        .iter()
        .find(|(_, max_alignment, max_slip)| t.alignment_deg <= *max_alignment && t.slip_deg <= *max_slip)
        .map_or(Grade::Bail, |(grade, _, _)| *grade)
}

pub fn base_points(tricks: &[Trick]) -> u32 {
    tricks.iter().map(|t| t.points).sum()
}

/// Chain multiplier: 1 trick ×1, 2 ×1.5, 3 ×2.25, ...
pub fn combo_multiplier(trick_count: u32) -> f64 {
    COMBO_STEP.powi(trick_count.saturating_sub(1) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flight(f: Flight) -> Flight {
        Flight { airtime: 1.0, max_height: 5.0, ..f }
    }
    fn names(tricks: &[Trick]) -> Vec<&str> {
        tricks.iter().map(|t| t.name.as_str()).collect()
    }
    fn touchdown(alignment_deg: f64, slip_deg: f64) -> Touchdown {
        Touchdown { alignment_deg, slip_deg, spin_rate: 0.0 }
    }

    #[test]
    fn a_rotation_30_degrees_short_still_counts() {
        assert_eq!(count_rotations(TAU - 30f64.to_radians(), TAU), 1);
    }

    #[test]
    fn a_rotation_60_degrees_short_does_not_count() {
        assert_eq!(count_rotations(TAU - 60f64.to_radians(), TAU), 0);
    }

    #[test]
    fn nose_down_is_a_frontflip_nose_up_a_backflip() {
        assert_eq!(names(&describe_tricks(&flight(Flight { pitch: -TAU, ..default() }))), ["Frontflip"]);
        assert_eq!(names(&describe_tricks(&flight(Flight { pitch: TAU, ..default() }))), ["Backflip"]);
    }

    #[test]
    fn repeated_rotations_get_a_prefix_and_multiply_points() {
        let tricks = describe_tricks(&flight(Flight { pitch: 2.0 * TAU, ..default() }));
        assert_eq!(tricks, [Trick { kind: TrickKind::Rotation, name: "Double Backflip".into(), points: 2000 }]);
    }

    #[test]
    fn yaw_is_scored_in_180s() {
        assert_eq!(names(&describe_tricks(&flight(Flight { yaw: 540f64.to_radians(), ..default() }))), ["540"]);
    }

    #[test]
    fn combined_tricks_are_listed_together() {
        let f = flight(Flight { pitch: -TAU, roll: TAU, yaw: TAU, ..default() });
        assert_eq!(names(&describe_tricks(&f)), ["Frontflip", "Barrel Roll", "360"]);
    }

    #[test]
    fn hang_time_and_height_are_bonus_tricks() {
        let tricks = describe_tricks(&Flight { roll: TAU, airtime: 3.2, max_height: 40.0, ..default() });
        assert_eq!(names(&tricks), ["Barrel Roll", "Big Air 3.2s", "High Flyer 40 m"]);
        let kinds: Vec<_> = tricks.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, [TrickKind::Rotation, TrickKind::Bonus, TrickKind::Bonus]);
    }

    #[test]
    fn a_short_flat_hop_earns_nothing() {
        assert!(describe_tricks(&flight(Flight::default())).is_empty());
    }

    #[test]
    fn landing_grades_step_down_with_alignment_and_slip() {
        assert_eq!(grade_landing(&touchdown(5.0, 5.0)), Grade::Perfect);
        assert_eq!(grade_landing(&touchdown(20.0, 5.0)), Grade::Clean);
        assert_eq!(grade_landing(&touchdown(5.0, 30.0)), Grade::Clean);
        assert_eq!(grade_landing(&touchdown(40.0, 5.0)), Grade::Sketchy);
        assert_eq!(grade_landing(&touchdown(5.0, 120.0)), Grade::Sketchy);
        assert_eq!(grade_landing(&touchdown(80.0, 0.0)), Grade::Bail);
    }

    #[test]
    fn touching_down_while_still_spinning_is_a_bail_however_level() {
        assert_eq!(grade_landing(&Touchdown { spin_rate: 5.0, ..touchdown(0.0, 0.0) }), Grade::Bail);
        assert_eq!(grade_landing(&Touchdown { spin_rate: 1.0, ..touchdown(0.0, 0.0) }), Grade::Perfect);
    }

    #[test]
    fn the_combo_multiplier_compounds_by_one_and_a_half() {
        let m: Vec<f64> = [1, 2, 3, 5].map(combo_multiplier).to_vec();
        assert_eq!(m, [1.0, 1.5, 2.25, 5.0625]);
    }

    fn default() -> Flight {
        Flight::default()
    }
}
