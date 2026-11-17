pub mod combo;
pub mod scoring;
pub mod tracker;

use bevy::math::DVec3;
use bevy::prelude::*;

use crate::rover::body::RoverBody;
use crate::rover::{Respawned, Rover, RoverSystems};
use crate::terrain::Terrain;
use crate::terrain::heightfield::Heightfield;
use tracker::{FlightSample, TrickEvent, TrickTracker};

const MIN_SLIP_SPEED: f64 = 3.0; // m/s; below this, heading vs travel is noise

#[derive(Resource, Default)]
pub struct Tricks(pub TrickTracker);

/// A scoring event for the HUD.
#[derive(Message)]
pub struct TrickMessage(pub TrickEvent);

pub struct TricksPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrickSystems;

impl Plugin for TricksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tricks>().add_message::<TrickMessage>().add_systems(Update, (reset_on_respawn, track).chain().in_set(TrickSystems).after(RoverSystems));
    }
}

fn reset_on_respawn(mut respawns: MessageReader<Respawned>, mut tricks: ResMut<Tricks>) {
    if respawns.read().count() > 0 {
        tricks.0.reset();
    }
}

fn track(time: Res<Time>, rover: Res<Rover>, terrain: Res<Terrain>, mut tricks: ResMut<Tricks>, mut messages: MessageWriter<TrickMessage>) {
    let sample = sample_flight(&rover.body, &terrain.heightfield);
    if let Some(event) = tricks.0.update(time.delta_secs_f64(), &sample) {
        messages.write(TrickMessage(event));
    }
}

/// Reduces rover state to the plain numbers the tracker scores.
fn sample_flight(body: &RoverBody, heightfield: &Heightfield) -> FlightSample {
    let p = body.position;
    let normal = heightfield.normal_at(p.x, p.z);
    FlightSample {
        airborne: !body.in_contact(),
        local_spin: body.rotation.inverse() * body.angular_velocity,
        height_above_ground: p.y - heightfield.height_at(p.x, p.z),
        alignment_deg: body.up().angle_between(normal).to_degrees(),
        slip_deg: slip_angle(body).to_degrees(),
    }
}

fn slip_angle(body: &RoverBody) -> f64 {
    let travel = body.velocity.with_y(0.0);
    if travel.length() < MIN_SLIP_SPEED {
        return 0.0;
    }
    let facing = body.forward().with_y(0.0);
    if facing == DVec3::ZERO { std::f64::consts::PI } else { facing.angle_between(travel) }
}
