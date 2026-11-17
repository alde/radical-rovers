//! Floating origin. Gameplay positions are f64 metres on the (shrunk) planet,
//! up to ~200 km from zero, where f32 only resolves ~1.5 cm. Rendering
//! happens relative to an origin that re-centres near the player.

use bevy::math::DVec3;
use bevy::prelude::*;

/// Recentre once the focus drifts this far from the render origin.
const RECENTRE_DISTANCE: f64 = 2_000.0;

#[derive(Component, Clone, Copy, Default)]
pub struct WorldPosition(pub DVec3);

#[derive(Resource, Default)]
pub struct WorldOrigin(pub DVec3);

impl WorldOrigin {
    pub fn to_render(&self, world: DVec3) -> Vec3 {
        (world - self.0).as_vec3()
    }
}

/// What the origin follows.
#[derive(Component)]
pub struct OriginFocus;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldOrigin>().add_systems(PostUpdate, (recentre_origin, apply_world_positions).chain().before(TransformSystems::Propagate));
    }
}

fn recentre_origin(mut origin: ResMut<WorldOrigin>, focus: Query<&WorldPosition, With<OriginFocus>>) {
    let Ok(focus) = focus.single() else { return };
    if focus.0.distance(origin.0) > RECENTRE_DISTANCE {
        origin.0 = focus.0.round();
    }
}

fn apply_world_positions(origin: Res<WorldOrigin>, mut query: Query<(Ref<WorldPosition>, &mut Transform)>) {
    let origin_moved = origin.is_changed();
    for (position, mut transform) in &mut query {
        if origin_moved || position.is_changed() {
            transform.translation = origin.to_render(position.0);
        }
    }
}
