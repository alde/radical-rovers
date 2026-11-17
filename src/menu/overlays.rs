//! How-to-play sheet and pause menu, drawn over whatever is behind.

use bevy::prelude::*;

use super::MenuAction;
use super::widgets::{chrome_text, neon_button};
use crate::audio::SoundSettings;
use crate::screen::Screen;
use crate::style::{self, MONO, NEON_CYAN, NEON_PINK, NIGHT, SHEET};

const CONTROLS: &str = "\
W / S        drive, brake, reverse
A / D        steer
Shift        boost (rockets fire in the air)
Space        handbrake
R            reset upright
G            gravity ×1 / ×2 / ×4
1-9          teleport to a landmark
Esc          pause";

const AIR: &str = "\
↑ ↓          pitch: front / back flips
← →          roll: barrel rolls
Q / E        yaw: spins
hands off    the gyro lines you up";

const SCORING: &str = "\
Flips, rolls, spins, big air and height
all score points.

Stop rotating before touchdown and land
straight:
  PERFECT ×3   CLEAN ×2   SKETCHY ×1   BAIL ×0";

const COMBOS: &str = "\
Land a trick and do another within 2.5 s
to chain them. Every trick multiplies the
chain by ×1.5. It banks when you stop.
Bail, and it's gone.";

fn overlay(screen: Screen) -> impl Bundle {
    let node = Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };
    (node, BackgroundColor(NIGHT.with_alpha(0.82)), DespawnOnExit(screen), GlobalZIndex(10))
}

pub fn spawn_how_to(mut commands: Commands, screen: Res<State<Screen>>) {
    commands.spawn(overlay(*screen.get())).with_children(|root| {
        let sheet = Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, padding: UiRect::axes(Val::Px(36.0), Val::Px(28.0)), border: UiRect::all(Val::Px(2.0)), ..default() };
        root.spawn((sheet, BackgroundColor(SHEET), BorderColor::all(NEON_PINK), style::glow(NEON_PINK.with_alpha(0.5), 30.0))).with_children(|sheet| {
            chrome_text(sheet, "HOW TO PLAY", 56.0);
            sheet.spawn(Node { column_gap: Val::Px(40.0), margin: UiRect::vertical(Val::Px(18.0)), ..default() }).with_children(|columns| {
                columns.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|col| {
                    section(col, "DRIVE", CONTROLS);
                    section(col, "IN THE AIR", AIR);
                });
                columns.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|col| {
                    section(col, "SCORE", SCORING);
                    section(col, "COMBOS", COMBOS);
                });
            });
            neon_button(sheet, "Back", MenuAction::Back, 0, true);
        });
    });
}

fn section(column: &mut ChildSpawnerCommands, heading: &str, body: &str) {
    column.spawn((style::display_text(heading, 16.0, NEON_CYAN), Node { margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(12.0), Val::Px(6.0)), ..default() }));
    column.spawn((Text::new(body), style::font(MONO, 13.0, false), TextColor(Color::WHITE)));
}

pub fn spawn_pause(mut commands: Commands, sound: Res<SoundSettings>) {
    commands.spawn(overlay(Screen::Paused)).with_children(|root| {
        chrome_text(root, "PAUSED", 104.0);
        root.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(14.0), margin: UiRect::top(Val::Px(24.0)), ..default() }).with_children(|buttons| {
            neon_button(buttons, "Resume", MenuAction::Resume, 0, true);
            neon_button(buttons, "How to play", MenuAction::HowTo, 1, false);
            neon_button(buttons, &sound_label(sound.enabled), MenuAction::ToggleSound, 2, false);
            neon_button(buttons, "Main menu", MenuAction::MainMenu, 3, false);
        });
    });
}

fn sound_label(enabled: bool) -> String {
    format!("SOUND: {}", if enabled { "ON" } else { "OFF" })
}

/// Keep the pause menu's sound button in step with the setting.
pub fn refresh_sound_label(sound: Res<SoundSettings>, buttons: Query<(&super::widgets::OnActivate, &Children)>, mut texts: Query<&mut Text>) {
    if !sound.is_changed() {
        return;
    }
    for (action, children) in &buttons {
        if action.0 != MenuAction::ToggleSound {
            continue;
        }
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child) {
                text.0 = sound_label(sound.enabled);
            }
        }
    }
}
