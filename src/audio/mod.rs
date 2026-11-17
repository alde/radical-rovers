//! Menu music and trick chimes, synthesised at startup. One switch mutes all.

mod synth;

use std::sync::Arc;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::screen::Screen;
use crate::tricks::scoring::Grade;
use crate::tricks::tracker::TrickEvent;
use crate::tricks::TrickMessage;

const MUSIC_VOLUME: f32 = 0.45;
const CHIME_VOLUME: f32 = 0.6;

#[derive(Resource)]
pub struct SoundSettings {
    pub enabled: bool,
}

#[derive(Resource)]
struct Sounds {
    music: Handle<AudioSource>,
    perfect: Handle<AudioSource>,
    clean: Handle<AudioSource>,
    sketchy: Handle<AudioSource>,
    bail: Handle<AudioSource>,
    bank: Handle<AudioSource>,
    trick: Handle<AudioSource>,
}

#[derive(Component)]
struct MenuMusic;

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SoundSettings { enabled: true })
            .add_systems(Startup, synthesise)
            .add_systems(Update, (play_menu_music, play_trick_chimes, apply_volume));
    }
}

fn source(samples: &[f32]) -> AudioSource {
    AudioSource { bytes: Arc::from(synth::wav_bytes(samples)) }
}

// C major arpeggios, in Hz.
const C5: f64 = 523.25;
const E5: f64 = 659.25;
const G5: f64 = 783.99;
const C6: f64 = 1046.5;

fn synthesise(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let mut add = |samples: Vec<f32>| sources.add(source(&samples));
    commands.insert_resource(Sounds {
        music: add(synth::menu_loop()),
        perfect: add(synth::chime(&[C5, E5, G5, C6], false)),
        clean: add(synth::chime(&[C5, E5, G5], false)),
        sketchy: add(synth::chime(&[392.0, 440.0], false)),
        bail: add(synth::chime(&[110.0, 82.4], true)),
        bank: add(synth::chime(&[G5, C6, 1318.5, 1568.0, 2093.0], false)),
        trick: add(synth::chime(&[880.0], false)),
    });
}

/// Music plays on the title and its how-to sheet, not during a run.
fn play_menu_music(mut commands: Commands, screen: Res<State<Screen>>, sounds: Res<Sounds>, music: Query<Entity, With<MenuMusic>>) {
    let wanted = !screen.get().shows_world();
    match (wanted, music.single()) {
        (true, Err(_)) => {
            commands.spawn((MenuMusic, AudioPlayer::new(sounds.music.clone()), PlaybackSettings::LOOP.with_volume(Volume::Linear(MUSIC_VOLUME))));
        }
        (false, Ok(entity)) => commands.entity(entity).despawn(),
        _ => {}
    }
}

fn play_trick_chimes(mut commands: Commands, mut messages: MessageReader<TrickMessage>, sounds: Res<Sounds>, mut last_trick_count: Local<usize>) {
    for TrickMessage(event) in messages.read() {
        let sound = match event {
            TrickEvent::Air { tricks, .. } => {
                let new_trick = tricks.len() > *last_trick_count;
                *last_trick_count = tricks.len();
                new_trick.then_some(&sounds.trick)
            }
            TrickEvent::Landed { grade, .. } => Some(match grade {
                Grade::Perfect => &sounds.perfect,
                Grade::Clean => &sounds.clean,
                _ => &sounds.sketchy,
            }),
            TrickEvent::Bailed { .. } => Some(&sounds.bail),
            TrickEvent::Banked(combo) => (combo.value > 0).then_some(&sounds.bank),
        };
        if !matches!(event, TrickEvent::Air { .. }) {
            *last_trick_count = 0;
        }
        if let Some(sound) = sound {
            commands.spawn((AudioPlayer::new(sound.clone()), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(CHIME_VOLUME))));
        }
    }
}

fn apply_volume(settings: Res<SoundSettings>, mut global: ResMut<GlobalVolume>) {
    if settings.is_changed() {
        global.volume = Volume::Linear(if settings.enabled { 1.0 } else { 0.0 });
    }
}
