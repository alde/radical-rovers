//! `--screenshot <path>`: save one frame once the terrain has streamed in,
//! then quit. For README images and checking a build without playing it.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};

use crate::terrain::chunks::ChunkCache;

/// Let the rover settle and the terrain finish streaming first.
const MIN_WAIT_SECONDS: f32 = 4.0;
const MAX_WAIT_SECONDS: f32 = 30.0;

#[derive(Resource)]
pub struct CaptureRequest {
    path: PathBuf,
    taken: bool,
}

pub struct CapturePlugin(pub PathBuf);

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CaptureRequest { path: self.0.clone(), taken: false }).add_systems(Update, capture_when_ready);
    }
}

fn capture_when_ready(mut commands: Commands, time: Res<Time>, cache: Res<ChunkCache>, mut request: ResMut<CaptureRequest>) {
    let elapsed = time.elapsed_secs();
    let ready = elapsed > MIN_WAIT_SECONDS && cache.pending == 0;
    if request.taken || !(ready || elapsed > MAX_WAIT_SECONDS) {
        return;
    }
    request.taken = true;
    commands.spawn(Screenshot::primary_window()).observe(save_to_disk(request.path.clone())).observe(
        |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            exit.write(AppExit::Success);
        },
    );
}
