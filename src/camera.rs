//! Chase camera smoothed in rover-relative space, so it never trails hundreds
//! of metres behind at boost speed.

use bevy::math::DVec3;
use bevy::prelude::*;

use crate::planet::Planet;
use crate::rover::{Respawned, Rover, RoverSystems};
use crate::terrain::Terrain;
use crate::world::WorldPosition;

const BASE_DISTANCE: f64 = 9.0;
const DISTANCE_PER_SPEED: f64 = 0.025;
const MAX_DISTANCE: f64 = 22.0;
const BASE_HEIGHT: f64 = 3.2;
const HEIGHT_PER_SPEED: f64 = 0.006;
const BASE_FOV_DEG: f32 = 65.0;
const MAX_EXTRA_FOV_DEG: f32 = 25.0; // speed sensation via FOV widening
const FOV_FULL_SPEED: f64 = 400.0;
const FOLLOW_RATE: f64 = 5.0; // 1/s
const HEADING_RATE: f64 = 3.0; // 1/s
const MIN_CLEARANCE: f64 = 1.5;
const LOOK_AHEAD: f64 = 4.0;
const LOOK_HEIGHT: f64 = 1.2;
const FAR_PLANE: f32 = 150_000.0;

#[derive(Component)]
pub struct ChaseCamera {
    offset: DVec3,
    heading: DVec3,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera).add_systems(Update, (snap_on_respawn, follow).chain().after(RoverSystems));
    }
}

fn spawn_camera(mut commands: Commands, planet: Res<Planet>) {
    let (fog_color, fog_start, fog_end) = planet.fog;
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection { fov: BASE_FOV_DEG.to_radians(), far: FAR_PLANE, near: 0.3, ..default() }),
        DistanceFog { color: fog_color, falloff: FogFalloff::Linear { start: fog_start, end: fog_end }, ..default() },
        ChaseCamera { offset: DVec3::new(0.0, BASE_HEIGHT, BASE_DISTANCE), heading: DVec3::NEG_Z },
        WorldPosition::default(),
        Transform::default(),
    ));
}

fn desired_offset(heading: DVec3, speed: f64) -> DVec3 {
    let distance = (BASE_DISTANCE + speed * DISTANCE_PER_SPEED).min(MAX_DISTANCE);
    (heading * -distance).with_y(BASE_HEIGHT + speed * HEIGHT_PER_SPEED)
}

fn snap_on_respawn(mut respawns: MessageReader<Respawned>, rover: Res<Rover>, mut camera: Query<&mut ChaseCamera>) {
    if respawns.read().count() == 0 {
        return;
    }
    let Ok(mut chase) = camera.single_mut() else { return };
    chase.heading = rover.body.forward().with_y(0.0).normalize_or(DVec3::NEG_Z);
    chase.offset = desired_offset(chase.heading, 0.0);
}

fn follow(time: Res<Time>, rover: Res<Rover>, terrain: Res<Terrain>, mut camera: Query<(&mut ChaseCamera, &mut WorldPosition, &mut Transform, &mut Projection)>) {
    let Ok((mut chase, mut position, mut transform, mut projection)) = camera.single_mut() else { return };
    let (body, dt) = (&rover.body, time.delta_secs_f64());
    let speed = body.speed();
    let steer = if speed > 3.0 { body.velocity.with_y(0.0) } else { body.forward().with_y(0.0) }.normalize_or_zero();
    if steer != DVec3::ZERO {
        chase.heading = chase.heading.lerp(steer, 1.0 - (-HEADING_RATE * dt).exp()).normalize();
    }
    let target_offset = desired_offset(chase.heading, speed);
    chase.offset = chase.offset.lerp(target_offset, 1.0 - (-FOLLOW_RATE * dt).exp());
    let mut eye = body.position + chase.offset;
    eye.y = eye.y.max(terrain.heightfield.height_at(eye.x, eye.z) + MIN_CLEARANCE);
    position.0 = eye;
    let target = (body.position + chase.heading * LOOK_AHEAD).with_y(body.position.y + LOOK_HEIGHT);
    transform.rotation = Transform::IDENTITY.looking_to((target - eye).as_vec3(), Vec3::Y).rotation;
    if let Projection::Perspective(p) = projection.as_mut() {
        p.fov = (BASE_FOV_DEG + MAX_EXTRA_FOV_DEG * (speed / FOV_FULL_SPEED).min(1.0) as f32).to_radians();
    }
}
