//! Arcade-tuned rover. Body frame: +x right, +y up, -z forward. Units SI.

use bevy::math::DVec3;

pub struct Gear {
    pub top_speed: f64,
    pub accel: f64,
}

pub struct RoverSpec {
    pub mass: f64,
    pub half_extents: DVec3,
    pub wheel_mounts: [DVec3; 4],
    pub wheel_radius: f64,
    pub wheel_width: f64,
    pub rest_length: f64,
    pub spring_k: f64,
    pub spring_damping: f64,
    pub hull_k: f64,
    pub hull_damping: f64,
    pub hull_friction: f64,
    pub max_steer: f64,
    /// Speed (m/s) at which steering lock halves.
    pub steer_falloff_speed: f64,
    /// Rear wheels counter-steer, like real rover corner drives.
    pub rear_steer_ratio: f64,
    /// 1/s: time constant for killing sideways slip.
    pub lateral_grip: f64,
    pub max_lateral_accel: f64,
    pub cruise: Gear,
    pub boost: Gear,
    pub reverse: Gear,
    /// m/s^2 from the rockets while boosting airborne.
    pub air_thrust: f64,
    pub brake_decel: f64,
    pub rolling_resistance: f64,
    pub air_pitch_accel: f64,
    pub air_roll_accel: f64,
    pub air_yaw_accel: f64,
    pub air_level_stiffness: f64,
    pub air_level_damping: f64,
    /// rad/s^2 per unit sin(yaw error).
    pub air_weathervane: f64,
    pub angular_damping: f64,
}

pub const ROVER: RoverSpec = RoverSpec {
    mass: 900.0,
    half_extents: DVec3::new(1.0, 0.4, 1.5),
    wheel_mounts: [
        DVec3::new(-1.2, -0.15, -1.25),
        DVec3::new(1.2, -0.15, -1.25),
        DVec3::new(-1.2, -0.15, 1.25),
        DVec3::new(1.2, -0.15, 1.25),
    ],
    wheel_radius: 0.5,
    wheel_width: 0.45,
    rest_length: 0.7,
    spring_k: 45_000.0,     // ~2.2 Hz per corner
    spring_damping: 3_500.0, // ~0.5 damping ratio
    hull_k: 300_000.0,
    hull_damping: 15_000.0,
    hull_friction: 0.5,
    max_steer: 0.55,
    steer_falloff_speed: 25.0,
    rear_steer_ratio: 0.5,
    lateral_grip: 8.0,
    max_lateral_accel: 30.0,
    cruise: Gear { top_speed: 70.0, accel: 16.0 },
    boost: Gear { top_speed: 420.0, accel: 80.0 },
    reverse: Gear { top_speed: 15.0, accel: 8.0 },
    air_thrust: 30.0,
    brake_decel: 35.0,
    rolling_resistance: 0.05,
    air_pitch_accel: 5.0,
    air_roll_accel: 6.0,
    air_yaw_accel: 3.0,
    air_level_stiffness: 6.0,
    air_level_damping: 3.0,
    air_weathervane: 8.0,
    angular_damping: 0.3,
};
