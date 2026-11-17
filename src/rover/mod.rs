pub mod body;
pub mod model;
pub mod spec;

use bevy::math::DVec3;
use bevy::prelude::*;

use crate::LaunchOptions;
use crate::planet::Planet;
use crate::terrain::Terrain;
use crate::terrain::chunks::TerrainFocus;
use crate::world::{OriginFocus, WorldPosition};
use body::{Controls, RoverBody};
use model::{WheelPivot, WheelTyre, advance_spin, spawn_model};
use spec::ROVER;

const FIXED_DT: f64 = 1.0 / 300.0; // physics rate; at 420 m/s that's 1.4 m per step
const MAX_SUBSTEPS: usize = 15;
const SPAWN_DROP: f64 = 3.0; // metres above ground on teleport/reset
const GRAVITY_SCALES: [f64; 3] = [1.0, 2.0, 4.0];
const STUCK_UP_THRESHOLD: f64 = 0.2; // on its roof...
const STUCK_SPEED: f64 = 3.0; // ...going nowhere...
const STUCK_SECONDS: f64 = 2.5; // ...for this long, and it respawns

#[derive(Resource)]
pub struct Rover {
    pub body: RoverBody,
    pub controls: Controls,
    pub gravity_index: usize,
    accumulator: f64,
    stuck_for: f64,
}

impl Rover {
    pub fn gravity_scale(&self) -> f64 {
        GRAVITY_SCALES[self.gravity_index]
    }
}

/// Fired when the rover is teleported or reset, so trick tracking can react.
#[derive(Message)]
pub struct Respawned;

/// Request to drop the rover at a landmark.
#[derive(Message)]
pub struct TeleportTo {
    pub lat: f64,
    pub lon: f64,
    pub name: &'static str,
}

#[derive(Component)]
pub struct RoverRoot;

pub struct RoverPlugin;

impl Plugin for RoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Respawned>()
            .add_message::<TeleportTo>()
            .insert_resource(Rover { body: RoverBody::new(&ROVER), controls: Controls::default(), gravity_index: 0, accumulator: 0.0, stuck_for: 0.0 })
            .add_systems(Startup, spawn_rover)
            .add_systems(Update, (read_controls, handle_teleports, handle_toggles, step_physics, recover_if_stuck, sync_model).chain().in_set(RoverSystems));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoverSystems;

fn spawn_rover(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    planet: Res<Planet>,
    launch: Res<LaunchOptions>,
    mut teleports: MessageWriter<TeleportTo>,
) {
    let root = commands.spawn((RoverRoot, WorldPosition::default(), OriginFocus, TerrainFocus, Transform::default(), Visibility::default())).id();
    spawn_model(&mut commands, root, &ROVER, &mut meshes, &mut materials);
    let site = &planet.landmarks[launch.site.min(planet.landmarks.len() - 1)];
    teleports.write(TeleportTo { lat: site.lat, lon: site.lon, name: site.name });
}

fn read_controls(keys: Res<ButtonInput<KeyCode>>, mut rover: ResMut<Rover>) {
    let axis = |pos: KeyCode, neg: KeyCode| f64::from(i8::from(keys.pressed(pos)) - i8::from(keys.pressed(neg)));
    rover.controls = Controls {
        throttle: axis(KeyCode::KeyW, KeyCode::KeyS),
        steer: axis(KeyCode::KeyD, KeyCode::KeyA),
        pitch: axis(KeyCode::ArrowUp, KeyCode::ArrowDown),
        roll: axis(KeyCode::ArrowRight, KeyCode::ArrowLeft),
        yaw: axis(KeyCode::KeyE, KeyCode::KeyQ),
        boost: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        handbrake: keys.pressed(KeyCode::Space),
    };
}

fn drop_at(rover: &mut Rover, terrain: &Terrain, x: f64, z: f64, respawned: &mut MessageWriter<Respawned>) {
    let y = terrain.heightfield.height_at(x, z) + SPAWN_DROP + ROVER.rest_length;
    let heading = rover.body.heading();
    rover.body.place_at(DVec3::new(x, y, z), heading);
    rover.stuck_for = 0.0;
    respawned.write(Respawned);
}

fn handle_teleports(mut requests: MessageReader<TeleportTo>, mut rover: ResMut<Rover>, terrain: Res<Terrain>, mut respawned: MessageWriter<Respawned>) {
    for request in requests.read() {
        let (x, z) = terrain.heightfield.from_lat_lon(request.lat, request.lon);
        drop_at(&mut rover, &terrain, x, z, &mut respawned);
    }
}

fn handle_toggles(keys: Res<ButtonInput<KeyCode>>, mut rover: ResMut<Rover>, terrain: Res<Terrain>, planet: Res<Planet>, mut respawned: MessageWriter<Respawned>, mut teleports: MessageWriter<TeleportTo>) {
    if keys.just_pressed(KeyCode::KeyR) {
        let p = rover.body.position;
        drop_at(&mut rover, &terrain, p.x, p.z, &mut respawned);
    }
    if keys.just_pressed(KeyCode::KeyG) {
        rover.gravity_index = (rover.gravity_index + 1) % GRAVITY_SCALES.len();
    }
    // Number keys jump to landmarks until the menus exist.
    for (i, key) in DIGIT_KEYS.iter().enumerate() {
        if keys.just_pressed(*key) && let Some(site) = planet.landmarks.get(i) {
            teleports.write(TeleportTo { lat: site.lat, lon: site.lon, name: site.name });
        }
    }
}

const DIGIT_KEYS: [KeyCode; 9] = [
    KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5,
    KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
];

fn step_physics(time: Res<Time>, mut rover: ResMut<Rover>, terrain: Res<Terrain>, planet: Res<Planet>) {
    let gravity = planet.gravity * rover.gravity_scale();
    let controls = rover.controls;
    rover.accumulator += time.delta_secs_f64();
    let mut steps = 0;
    while rover.accumulator >= FIXED_DT && steps < MAX_SUBSTEPS {
        rover.body.step(FIXED_DT, &controls, terrain.heightfield.as_ref(), gravity);
        rover.accumulator -= FIXED_DT;
        steps += 1;
    }
    rover.accumulator = rover.accumulator.min(FIXED_DT);
}

fn recover_if_stuck(time: Res<Time>, mut rover: ResMut<Rover>, terrain: Res<Terrain>, mut respawned: MessageWriter<Respawned>) {
    let stuck = rover.body.up().y < STUCK_UP_THRESHOLD && rover.body.speed() < STUCK_SPEED;
    rover.stuck_for = if stuck { rover.stuck_for + time.delta_secs_f64() } else { 0.0 };
    if rover.stuck_for > STUCK_SECONDS {
        let p = rover.body.position;
        drop_at(&mut rover, &terrain, p.x, p.z, &mut respawned);
    }
}

fn sync_model(
    time: Res<Time>,
    rover: Res<Rover>,
    mut root: Query<(&mut WorldPosition, &mut Transform), With<RoverRoot>>,
    mut pivots: Query<(&WheelPivot, &mut Transform, &Children), Without<RoverRoot>>,
    mut tyres: Query<(&mut WheelTyre, &mut Transform), (Without<WheelPivot>, Without<RoverRoot>)>,
) {
    let body = &rover.body;
    let Ok((mut position, mut transform)) = root.single_mut() else { return };
    position.0 = body.position;
    transform.rotation = body.rotation.as_quat();
    let dt = time.delta_secs();
    for (pivot, mut pivot_transform, children) in &mut pivots {
        let wheel = &body.wheels[pivot.0];
        pivot_transform.translation = (wheel.mount - DVec3::Y * wheel.extension).as_vec3();
        pivot_transform.rotation = Quat::from_rotation_y(wheel.steer as f32);
        for child in children {
            if let Ok((mut tyre, mut tyre_transform)) = tyres.get_mut(*child) {
                let spin = advance_spin(&mut tyre, wheel.grounded, wheel.ground_speed as f32, ROVER.wheel_radius as f32, dt);
                tyre_transform.rotate_local_x(spin);
            }
        }
    }
}
