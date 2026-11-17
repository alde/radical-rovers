//! Rigid-body rover over a heightfield: four spring-damper wheels with arcade
//! tyre forces, penalty contacts on the hull corners, air control and a
//! hands-off gyro. Pure f64 maths with no Bevy systems, so it tests headless.

use bevy::math::{DQuat, DVec3};

use super::spec::RoverSpec;
use crate::terrain::heightfield::Heightfield;

/// Below this |up . normal| the wheels can't touch (rolled over).
const MIN_WHEEL_ALIGNMENT: f64 = 0.2;
/// Grip ramps in over the first few mm. Low g means static sag is only ~8 mm
/// (Moon) to ~19 mm (Mars), so this must stay well below that.
const FULL_GRIP_COMPRESSION: f64 = 0.005;
const HANDBRAKE_RATE: f64 = 4.0; // 1/s
const REVERSE_THRESHOLD: f64 = 1.0; // m/s; above this, "back" brakes instead of reversing
const HULL_FRICTION_RATE: f64 = 6.0; // 1/s cap on how fast hull scraping can stop you
const MIN_WEATHERVANE_SPEED: f64 = 5.0; // m/s

/// What the rover drives on.
pub trait Ground {
    fn height_at(&self, x: f64, z: f64) -> f64;
    fn normal_at(&self, x: f64, z: f64) -> DVec3;
}

impl Ground for Heightfield {
    fn height_at(&self, x: f64, z: f64) -> f64 {
        Heightfield::height_at(self, x, z)
    }
    fn normal_at(&self, x: f64, z: f64) -> DVec3 {
        Heightfield::normal_at(self, x, z)
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Controls {
    pub throttle: f64,
    pub steer: f64,
    /// Air only: +1 nose down.
    pub pitch: f64,
    /// Air only: +1 roll right.
    pub roll: f64,
    /// Air only: +1 turn right.
    pub yaw: f64,
    pub boost: bool,
    pub handbrake: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct WheelState {
    pub mount: DVec3,
    pub steer: f64,
    pub extension: f64,
    pub ground_speed: f64,
    pub grounded: bool,
}

pub struct RoverBody {
    pub spec: &'static RoverSpec,
    pub position: DVec3,
    pub velocity: DVec3,
    pub rotation: DQuat,
    pub angular_velocity: DVec3,
    pub wheels: [WheelState; 4],
    pub grounded_wheels: usize,
    pub hull_contacts: usize,
    inv_inertia: DVec3,
    corners: [DVec3; 8],
    force: DVec3,
    torque: DVec3,
    air_accel: DVec3,
}

impl RoverBody {
    pub fn new(spec: &'static RoverSpec) -> Self {
        let wheel = |mount| WheelState { mount, steer: 0.0, extension: spec.rest_length, ground_speed: 0.0, grounded: false };
        Self {
            spec,
            position: DVec3::ZERO,
            velocity: DVec3::ZERO,
            rotation: DQuat::IDENTITY,
            angular_velocity: DVec3::ZERO,
            wheels: spec.wheel_mounts.map(wheel),
            grounded_wheels: 0,
            hull_contacts: 0,
            inv_inertia: box_inverse_inertia(spec.mass, spec.half_extents),
            corners: box_corners(spec.half_extents),
            force: DVec3::ZERO,
            torque: DVec3::ZERO,
            air_accel: DVec3::ZERO,
        }
    }

    pub fn up(&self) -> DVec3 { self.rotation * DVec3::Y }
    pub fn forward(&self) -> DVec3 { self.rotation * DVec3::NEG_Z }
    pub fn speed(&self) -> f64 { self.velocity.length() }
    pub fn forward_speed(&self) -> f64 { self.velocity.dot(self.forward()) }
    /// Touching the ground with anything: wheels or hull.
    pub fn in_contact(&self) -> bool { self.grounded_wheels > 0 || self.hull_contacts > 0 }

    pub fn heading(&self) -> f64 {
        let f = self.forward();
        (-f.x).atan2(-f.z)
    }

    pub fn place_at(&mut self, position: DVec3, heading: f64) {
        self.position = position;
        self.rotation = DQuat::from_rotation_y(heading);
        self.velocity = DVec3::ZERO;
        self.angular_velocity = DVec3::ZERO;
    }

    pub fn step(&mut self, dt: f64, controls: &Controls, ground: &impl Ground, gravity: f64) {
        self.force = DVec3::new(0.0, -gravity * self.spec.mass, 0.0);
        self.torque = DVec3::ZERO;
        self.air_accel = DVec3::ZERO;
        self.grounded_wheels = 0;
        self.hull_contacts = 0;
        for i in 0..self.wheels.len() {
            self.apply_wheel(i, controls, ground);
        }
        for i in 0..self.corners.len() {
            self.apply_hull_contact(self.corners[i], ground);
        }
        if self.grounded_wheels == 0 {
            self.apply_air_control(controls, ground);
            if controls.boost && controls.throttle > 0.0 {
                self.apply_rocket();
            }
        }
        self.integrate(dt);
    }

    fn apply_wheel(&mut self, i: usize, controls: &Controls, ground: &impl Ground) {
        let s = self.spec;
        let reach = s.rest_length + s.wheel_radius;
        let mount = self.to_world(self.wheels[i].mount);
        let n = ground.normal_at(mount.x, mount.z);
        let down = -self.up();
        let alignment = down.dot(n);
        let wheel = &mut self.wheels[i];
        wheel.grounded = false;
        wheel.extension = s.rest_length;
        if alignment > -MIN_WHEEL_ALIGNMENT {
            return;
        }
        // Ray along the suspension axis against the local ground plane.
        let t = (ground.height_at(mount.x, mount.z) - mount.y) * n.y / alignment;
        wheel.extension = (t - s.wheel_radius).clamp(0.0, s.rest_length);
        if t >= reach {
            return;
        }
        let compression = (reach - t).min(reach);
        let contact = mount + down * t;
        let pv = self.point_velocity(contact);
        let spring = (s.spring_k * compression - s.spring_damping * pv.dot(n)).max(0.0);
        self.add_force_at(n * spring, contact);
        self.apply_tyre(i, n, pv, compression, controls);
        self.wheels[i].grounded = true;
        self.grounded_wheels += 1;
    }

    // Tyre forces act at centre-of-mass height above the wheel, not at the
    // contact patch: an arcade trick so cornering yaws the rover but can't
    // lever it over. Slopes, kerbs and bad landings can still roll it.
    fn apply_tyre(&mut self, i: usize, n: DVec3, pv: DVec3, compression: f64, controls: &Controls) {
        let s = self.spec;
        let steer = self.steer_angle(i, controls);
        let up = self.up();
        let mut fwd = DQuat::from_axis_angle(up, steer) * self.forward();
        fwd = (fwd - n * fwd.dot(n)).normalize();
        let side = fwd.cross(n);
        let (v_long, v_lat) = (pv.dot(fwd), pv.dot(side));
        let load = (compression / FULL_GRIP_COMPRESSION).min(1.0) * (s.mass / self.wheels.len() as f64);
        let lateral = (-v_lat * s.lateral_grip).clamp(-s.max_lateral_accel, s.max_lateral_accel) * load;
        let longitudinal = self.longitudinal_accel(v_long, controls) * load;
        let mount = self.wheels[i].mount;
        let at = self.to_world(DVec3::new(mount.x, 0.0, mount.z));
        self.add_force_at(side * lateral + fwd * longitudinal, at);
        let wheel = &mut self.wheels[i];
        wheel.steer = steer;
        wheel.ground_speed = v_long;
    }

    fn steer_angle(&self, i: usize, controls: &Controls) -> f64 {
        let s = self.spec;
        let lock = s.max_steer / (1.0 + self.forward_speed().abs() / s.steer_falloff_speed);
        let is_front = self.wheels[i].mount.z < 0.0;
        -controls.steer * lock * if is_front { 1.0 } else { -s.rear_steer_ratio }
    }

    fn longitudinal_accel(&self, v_long: f64, c: &Controls) -> f64 {
        let s = self.spec;
        let gear = if c.boost { &s.boost } else { &s.cruise };
        if c.handbrake {
            -(v_long * HANDBRAKE_RATE).clamp(-s.brake_decel, s.brake_decel)
        } else if c.throttle > 0.0 {
            c.throttle * gear.accel * (1.0 - v_long / gear.top_speed).clamp(0.0, 1.0)
        } else if c.throttle < 0.0 && v_long > REVERSE_THRESHOLD {
            c.throttle * s.brake_decel
        } else if c.throttle < 0.0 {
            c.throttle * s.reverse.accel * (1.0 + v_long / s.reverse.top_speed).clamp(0.0, 1.0)
        } else {
            -v_long * s.rolling_resistance
        }
    }

    fn apply_hull_contact(&mut self, corner: DVec3, ground: &impl Ground) {
        let s = self.spec;
        let p = self.to_world(corner);
        let height = ground.height_at(p.x, p.z);
        if p.y >= height {
            return;
        }
        self.hull_contacts += 1;
        let n = ground.normal_at(p.x, p.z);
        let pv = self.point_velocity(p);
        let vn = pv.dot(n);
        let normal_force = (s.hull_k * (height - p.y) * n.y - s.hull_damping * vn).max(0.0);
        let tangential = pv - n * vn;
        let vt = tangential.length();
        let friction = if vt > 1e-3 { (s.hull_friction * normal_force).min(vt * s.mass * HULL_FRICTION_RATE) / vt } else { 0.0 };
        self.add_force_at(n * normal_force - tangential * friction, p);
    }

    // Boost keeps pushing in the air; at speed you spend most of your time there.
    fn apply_rocket(&mut self) {
        let s = self.spec;
        let headroom = (1.0 - self.forward_speed() / s.boost.top_speed).clamp(0.0, 1.0);
        self.force += self.forward() * s.air_thrust * s.mass * headroom;
    }

    fn apply_air_control(&mut self, c: &Controls, ground: &impl Ground) {
        let s = self.spec;
        let local = DVec3::new(-c.pitch * s.air_pitch_accel, -c.yaw * s.air_yaw_accel, -c.roll * s.air_roll_accel);
        self.air_accel = self.rotation * local;
        if c.pitch == 0.0 && c.roll == 0.0 && c.yaw == 0.0 {
            self.apply_air_assist(ground);
        }
    }

    // Hands off in the air, a gyro lines the rover up with the ground beneath
    // it and turns its nose into the direction of travel, so long jumps land
    // on the wheels and match the slope they come down on.
    fn apply_air_assist(&mut self, ground: &impl Ground) {
        let s = self.spec;
        let tilt = self.up().cross(ground.normal_at(self.position.x, self.position.z));
        self.air_accel += tilt * s.air_level_stiffness - self.angular_velocity * s.air_level_damping;
        let travel = self.velocity.with_y(0.0);
        if travel.length() < MIN_WEATHERVANE_SPEED {
            return;
        }
        let (facing, travel) = (self.forward().with_y(0.0).normalize_or_zero(), travel.normalize());
        if facing.dot(travel) > 0.0 {
            self.air_accel.y += facing.cross(travel).y * s.air_weathervane;
        }
    }

    fn integrate(&mut self, dt: f64) {
        self.velocity += self.force * (dt / self.spec.mass);
        self.position += self.velocity * dt;
        let angular_accel = self.world_inverse_inertia(self.torque) + self.air_accel;
        self.angular_velocity += angular_accel * dt;
        self.angular_velocity *= (1.0 - self.spec.angular_damping * dt).max(0.0);
        let w = self.angular_velocity * (dt * 0.5);
        let dq = DQuat::from_xyzw(w.x, w.y, w.z, 0.0) * self.rotation;
        let q = self.rotation;
        self.rotation = DQuat::from_xyzw(q.x + dq.x, q.y + dq.y, q.z + dq.z, q.w + dq.w).normalize();
    }

    fn to_world(&self, local: DVec3) -> DVec3 {
        self.position + self.rotation * local
    }

    fn add_force_at(&mut self, force: DVec3, point: DVec3) {
        self.force += force;
        self.torque += (point - self.position).cross(force);
    }

    fn point_velocity(&self, point: DVec3) -> DVec3 {
        self.velocity + self.angular_velocity.cross(point - self.position)
    }

    fn world_inverse_inertia(&self, torque: DVec3) -> DVec3 {
        self.rotation * ((self.rotation.inverse() * torque) * self.inv_inertia)
    }
}

fn box_inverse_inertia(mass: f64, e: DVec3) -> DVec3 {
    DVec3::new(3.0 / (mass * (e.y * e.y + e.z * e.z)), 3.0 / (mass * (e.x * e.x + e.z * e.z)), 3.0 / (mass * (e.x * e.x + e.y * e.y)))
}

fn box_corners(e: DVec3) -> [DVec3; 8] {
    std::array::from_fn(|k| {
        let sign = |bit: usize| if k & bit == 0 { -1.0 } else { 1.0 };
        DVec3::new(sign(4) * e.x, sign(2) * e.y, sign(1) * e.z)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rover::spec::ROVER;

    const DT: f64 = 1.0 / 300.0;
    const MARS_G: f64 = 3.71;

    struct Flat;
    impl Ground for Flat {
        fn height_at(&self, _: f64, _: f64) -> f64 { 0.0 }
        fn normal_at(&self, _: f64, _: f64) -> DVec3 { DVec3::Y }
    }

    fn settled() -> RoverBody {
        let mut body = RoverBody::new(&ROVER);
        body.place_at(DVec3::new(0.0, 2.0, 0.0), 0.0);
        run(&mut body, 3.0, Controls::default());
        body
    }

    fn run(body: &mut RoverBody, seconds: f64, controls: Controls) {
        for _ in 0..(seconds / DT) as usize {
            body.step(DT, &controls, &Flat, MARS_G);
        }
    }

    #[test]
    fn comes_to_rest_on_all_four_wheels() {
        let body = settled();
        assert_eq!(body.grounded_wheels, 4);
        assert_eq!(body.hull_contacts, 0);
        assert!(body.speed() < 0.05, "still moving at {}", body.speed());
        assert!(body.up().y > 0.999);
    }

    #[test]
    fn cruise_accelerates_forward_to_top_speed() {
        let mut body = settled();
        run(&mut body, 20.0, Controls { throttle: 1.0, ..default() });
        assert!((body.forward_speed() - ROVER.cruise.top_speed).abs() < 2.0, "speed {}", body.forward_speed());
        assert!(body.position.z < -500.0, "should drive towards -z");
    }

    #[test]
    fn steering_right_turns_right() {
        let mut body = settled();
        run(&mut body, 3.0, Controls { throttle: 1.0, steer: 1.0, ..default() });
        assert!(body.heading() < -0.3, "heading {}", body.heading());
        assert!(body.up().y > 0.95, "should stay upright");
    }

    #[test]
    fn handbrake_stops_it() {
        let mut body = settled();
        run(&mut body, 5.0, Controls { throttle: 1.0, ..default() });
        run(&mut body, 6.0, Controls { handbrake: true, ..default() });
        assert!(body.speed() < 0.5);
    }

    #[test]
    fn gyro_rights_a_tumbling_rover_in_the_air() {
        let mut body = RoverBody::new(&ROVER);
        body.place_at(DVec3::new(0.0, 500.0, 0.0), 0.0);
        body.angular_velocity = DVec3::new(3.0, 0.0, 2.0);
        run(&mut body, 4.0, Controls::default());
        assert!(body.up().y > 0.99, "up.y {}", body.up().y);
        assert!(body.angular_velocity.length() < 0.1);
    }

    #[test]
    fn pitch_input_flips_it_in_the_air() {
        let mut body = RoverBody::new(&ROVER);
        body.place_at(DVec3::new(0.0, 500.0, 0.0), 0.0);
        run(&mut body, 0.5, Controls { pitch: -1.0, ..default() });
        assert!(body.angular_velocity.x > 2.0, "nose-up spin {}", body.angular_velocity.x);
    }

    fn default() -> Controls {
        Controls::default()
    }
}
