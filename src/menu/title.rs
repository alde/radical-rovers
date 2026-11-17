//! Title screen: chrome-and-neon logo, a card per world, start buttons.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use super::widgets::{OnActivate, chrome_text, neon_button, send_activation};
use super::{MenuAction, MenuSelection, thumbnail};
use crate::screen::Screen;
use crate::style::{self, DISPLAY, HOT_PINK, MONO, NEON_CYAN, NEON_PINK, SCRIPT, SHEET};
use crate::worlds::Worlds;

const CARD_WIDTH: f32 = 290.0;
const LOGO_SIZE: f32 = 112.0;
const SCRIPT_SIZE: f32 = 92.0;
const SCRIPT_TILT_DEG: f32 = -6.0;
/// Offsets for the faked neon halo behind the script lettering.
const HALO_OFFSETS: [(f32, f32); 8] = [(-3.0, 0.0), (3.0, 0.0), (0.0, -3.0), (0.0, 3.0), (-2.0, -2.0), (2.0, 2.0), (-2.0, 2.0), (2.0, -2.0)];

#[derive(Component)]
pub struct WorldCard(usize);

#[derive(Component)]
pub struct SiteLabel(usize);

/// Builds the card maps on first entry (the first screen is entered before
/// any Startup system runs) and reuses them after.
pub fn spawn(mut commands: Commands, worlds: Res<Worlds>, mut images: ResMut<Assets<Image>>, mut thumbnails: Local<Vec<Handle<Image>>>) {
    if thumbnails.is_empty() {
        *thumbnails = worlds.0.iter().map(|w| images.add(thumbnail::thumbnail_image(&w.terrain))).collect();
    }
    let root = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };
    commands.spawn((root, DespawnOnExit(Screen::Title))).with_children(|screen| {
        spawn_logo(screen);
        screen.spawn((style::display_text("REAL PLANETS  ·  UNREAL SPEED", 16.0, NEON_CYAN), Node { margin: UiRect::vertical(Val::Px(14.0)), ..default() }));
        screen.spawn(Node { column_gap: Val::Px(22.0), margin: UiRect::bottom(Val::Px(22.0)), ..default() }).with_children(|cards| {
            for (i, world) in worlds.0.iter().enumerate() {
                spawn_card(cards, i, world, thumbnails[i].clone());
            }
        });
        screen.spawn(Node { column_gap: Val::Px(16.0), ..default() }).with_children(|buttons| {
            neon_button(buttons, "Start", MenuAction::Start, 0, true);
            neon_button(buttons, "How to play", MenuAction::HowTo, 1, false);
            neon_button(buttons, "Quit", MenuAction::Quit, 2, false);
        });
        screen.spawn((style::mono_text("← →  choose world   ·   ↑ ↓  landing site   ·   Enter  start", 12.0, Color::srgba(1.0, 1.0, 1.0, 0.5)), Node { margin: UiRect::top(Val::Px(16.0)), ..default() }));
        let credit = Node { margin: UiRect::top(Val::Px(10.0)), ..default() };
        screen.spawn((style::mono_text("Terrain: NASA MGS MOLA & LRO LOLA, PDS Geosciences Node", 11.0, Color::srgba(1.0, 1.0, 1.0, 0.45)), credit));
    });
}

fn spawn_logo(screen: &mut ChildSpawnerCommands) {
    screen.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }).with_children(|logo| {
        chrome_text(logo, "RADICAL", LOGO_SIZE);
        let script = Node { margin: UiRect::top(Val::Px(-SCRIPT_SIZE * 0.42)), position_type: PositionType::Relative, ..default() };
        logo.spawn((script, UiTransform { rotation: Rot2::degrees(SCRIPT_TILT_DEG), ..default() })).with_children(|neon| {
            let lettering = || style::font(SCRIPT, SCRIPT_SIZE, true);
            for (dx, dy) in HALO_OFFSETS {
                let at = Node { position_type: PositionType::Absolute, left: Val::Px(dx), top: Val::Px(dy), ..default() };
                neon.spawn((Text::new("Rovers"), lettering(), TextColor(NEON_PINK.with_alpha(0.28)), at));
            }
            neon.spawn((Text::new("Rovers"), lettering(), TextColor(HOT_PINK.mix(&Color::WHITE, 0.25))));
        });
    });
}

fn spawn_card(cards: &mut ChildSpawnerCommands, index: usize, world: &crate::worlds::World, thumbnail: Handle<Image>) {
    let planet = &world.planet;
    let lap_km = std::f64::consts::TAU * planet.radius / planet.map_scale / 1000.0;
    let card = Node {
        width: Val::Px(CARD_WIDTH),
        padding: UiRect::all(Val::Px(12.0)),
        border: UiRect::all(Val::Px(2.0)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(6.0),
        ..default()
    };
    cards
        .spawn((Button, Hovered::default(), WorldCard(index), OnActivate(MenuAction::SelectWorld(index)), card, BackgroundColor(SHEET), BorderColor::all(NEON_CYAN.with_alpha(0.35)), BoxShadow::default()))
        .observe(send_activation)
        .with_children(|card| {
            card.spawn((ImageNode::new(thumbnail), Node { width: Val::Percent(100.0), aspect_ratio: Some(2.0), ..default() }));
            card.spawn((Text::new(planet.name.to_uppercase()), style::font(DISPLAY, 28.0, true), TextColor(NEON_CYAN)));
            let stats = format!("GRAVITY  {} m/s²\nSCALE    1:{}\nLAP      {} km", planet.gravity, planet.map_scale, lap_km.round());
            card.spawn((Text::new(stats), style::font(MONO, 12.0, false), TextColor(Color::WHITE)));
            card.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(6.0), margin: UiRect::top(Val::Px(4.0)), ..default() }).with_children(|row| {
                site_arrow(row, "◀", index, -1);
                row.spawn((SiteLabel(index), Text::new(""), style::font(DISPLAY, 13.0, false), TextColor(Color::WHITE), Node { flex_grow: 1.0, ..default() }));
                site_arrow(row, "▶", index, 1);
            });
        });
}

fn site_arrow(row: &mut ChildSpawnerCommands, glyph: &str, world: usize, delta: i32) {
    row.spawn((Button, OnActivate(MenuAction::CycleSite { world, delta }), Node { padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)), ..default() }))
        .observe(send_activation)
        .with_child((Text::new(glyph), style::font(MONO, 14.0, false), TextColor(NEON_CYAN)));
}

/// Highlight the chosen world and show each card's landing site.
pub fn refresh_cards(
    selection: Res<MenuSelection>,
    worlds: Res<Worlds>,
    mut cards: Query<(&WorldCard, &mut BorderColor, &mut BoxShadow)>,
    mut labels: Query<(&SiteLabel, &mut Text)>,
) {
    for (card, mut border, mut shadow) in &mut cards {
        let chosen = card.0 == selection.world;
        *border = BorderColor::all(if chosen { NEON_PINK } else { NEON_CYAN.with_alpha(0.35) });
        *shadow = if chosen { style::glow(NEON_PINK.with_alpha(0.7), 24.0) } else { BoxShadow::default() };
    }
    for (label, mut text) in &mut labels {
        let site = &worlds.0[label.0].planet.landmarks[selection.sites[label.0]];
        text.0 = site.name.to_uppercase();
    }
}
