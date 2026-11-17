//! Sun, ambient light and sky colour per planet.

use bevy::prelude::*;

use crate::planet::Planet;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_sky);
    }
}

fn setup_sky(mut commands: Commands, planet: Res<Planet>) {
    commands.insert_resource(ClearColor(planet.sky));
    let (ambient_color, ambient_brightness) = planet.ambient;
    commands.insert_resource(GlobalAmbientLight { color: ambient_color, brightness: ambient_brightness, ..default() });
    commands.spawn((
        DirectionalLight { illuminance: planet.sun_illuminance, shadow_maps_enabled: false, ..default() },
        Transform::IDENTITY.looking_to(-planet.sun_direction.normalize(), Vec3::Y),
    ));
}
