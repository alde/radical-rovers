//! Speedo, position readout, score, combo meter, live trick text and popups.

use bevy::prelude::*;

use crate::planet::Planet;
use crate::rover::{Rover, TeleportTo};
use crate::terrain::Terrain;
use crate::tricks::scoring::{Grade, Trick};
use crate::tricks::tracker::TrickEvent;
use crate::tricks::{TrickMessage, Tricks};

const POPUP_SECONDS: f32 = 2.6;
const BANK_SECONDS: f32 = 1.6;
const FADE_SECONDS: f32 = 0.5;
const PANEL: Color = Color::srgba(0.04, 0.04, 0.06, 0.55);
const GOLD: Color = Color::srgb(1.0, 0.85, 0.29);
const GREEN: Color = Color::srgb(0.49, 0.88, 0.49);
const TAN: Color = Color::srgb(0.91, 0.77, 0.6);
const RED: Color = Color::srgb(1.0, 0.35, 0.29);

#[derive(Component)]
struct StatsText;
#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct ComboRow;
#[derive(Component)]
struct ComboText;
#[derive(Component)]
struct ComboTimer;
#[derive(Component)]
struct LiveTrickText;

/// A line of text that shows for a while, then fades.
#[derive(Component, Default)]
struct Flash {
    remaining: f32,
    color: Color,
}

#[derive(Component)]
enum PopupLine {
    Grade,
    Tricks,
    Points,
    Bank,
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud).add_systems(Update, (update_stats, update_score, show_trick_events, show_arrivals, fade_flashes).chain().after(crate::tricks::TrickSystems));
    }
}

/// System monospace: Bevy's built-in font lacks glyphs like ≈ ° ² ·.
const FONT_FAMILIES: &str = "Menlo, SF Mono, DejaVu Sans Mono, Consolas, monospace";

fn text(value: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    let font = TextFont { font: FontSource::Families(FONT_FAMILIES.into()), font_size: FontSize::Px(size), ..default() };
    (Text::new(value), font, TextColor(color))
}

fn spawn_hud(mut commands: Commands) {
    let panel = |left: Val, top: Val| Node { position_type: PositionType::Absolute, left, top, padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)), flex_direction: FlexDirection::Column, ..default() };
    commands.spawn((panel(Val::Px(12.0), Val::Px(12.0)), BackgroundColor(PANEL))).with_child((StatsText, text("", 14.0, Color::WHITE)));
    spawn_score_panel(&mut commands);
    let centred = |top: f32| Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), top: Val::Percent(top), justify_content: JustifyContent::Center, ..default() };
    commands.spawn(centred(22.0)).with_child((LiveTrickText, text("", 24.0, Color::WHITE)));
    commands.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..centred(30.0) }).with_children(|popup| {
        popup.spawn((PopupLine::Grade, Flash::default(), text("", 42.0, GOLD)));
        popup.spawn((PopupLine::Tricks, Flash::default(), text("", 19.0, Color::WHITE)));
        popup.spawn((PopupLine::Points, Flash::default(), text("", 32.0, GOLD)));
    });
    commands.spawn(centred(13.0)).with_child((PopupLine::Bank, Flash::default(), text("", 24.0, GOLD)));
}

fn spawn_score_panel(commands: &mut Commands) {
    let node = Node {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Percent(50.0),
        margin: UiRect::left(Val::Px(-90.0)),
        width: Val::Px(180.0),
        padding: UiRect::all(Val::Px(10.0)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        ..default()
    };
    commands.spawn((node, BackgroundColor(PANEL))).with_children(|panel| {
        panel.spawn(text("SCORE", 11.0, Color::srgba(1.0, 1.0, 1.0, 0.65)));
        panel.spawn((ScoreText, text("0", 28.0, Color::WHITE)));
        panel.spawn((ComboRow, Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, width: Val::Percent(100.0), display: Display::None, ..default() })).with_children(|row| {
            row.spawn((ComboText, text("", 18.0, GOLD)));
            row.spawn((Node { width: Val::Percent(100.0), height: Val::Px(4.0), margin: UiRect::top(Val::Px(4.0)), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15))))
                .with_child((ComboTimer, Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(GOLD)));
        });
    });
}

fn update_stats(rover: Res<Rover>, terrain: Res<Terrain>, planet: Res<Planet>, mut stats: Query<&mut Text, With<StatsText>>) {
    let Ok(mut text) = stats.single_mut() else { return };
    let body = &rover.body;
    let (lat, lon) = terrain.heightfield.to_lat_lon(body.position.x, body.position.z);
    let kmh = body.speed() * 3.6;
    let gravity = match rover.gravity_scale() {
        1.0 => format!("{} m/s²", planet.gravity),
        scale => format!("{} × {scale} m/s²", planet.gravity),
    };
    let elevation = terrain.heightfield.elevation_at(body.position.x, body.position.z);
    text.0 = format!(
        "{}\n{} km/h\n≈ {} km/h at true scale\nelev {} m\n{}  {}\ng {gravity}\n{} · scale 1:{}\n1-{} teleport · R reset · G gravity",
        planet.name,
        thousands(kmh),
        thousands(kmh * planet.map_scale),
        thousands(elevation),
        format_lat(lat),
        format_lon(lon),
        terrain.source_label,
        planet.map_scale,
        planet.landmarks.len().min(9),
    );
}

fn update_score(
    tricks: Res<Tricks>,
    mut score: Query<&mut Text, (With<ScoreText>, Without<ComboText>)>,
    mut row: Query<&mut Node, (With<ComboRow>, Without<ComboTimer>)>,
    mut combo_text: Query<&mut Text, (With<ComboText>, Without<ScoreText>)>,
    mut timer: Query<&mut Node, (With<ComboTimer>, Without<ComboRow>)>,
) {
    let tracker = &tricks.0;
    if let Ok(mut text) = score.single_mut() {
        text.0 = thousands(f64::from(tracker.score));
    }
    let combo = &tracker.combo;
    if let Ok(mut node) = row.single_mut() {
        node.display = if combo.active() { Display::Flex } else { Display::None };
    }
    if let Ok(mut text) = combo_text.single_mut() {
        text.0 = format!("{}  {}", format_multiplier(combo.multiplier()), thousands(f64::from(combo.value())));
    }
    if let Ok(mut node) = timer.single_mut() {
        node.width = Val::Percent(combo.time_left() as f32 * 100.0);
    }
}

fn show_trick_events(mut messages: MessageReader<TrickMessage>, mut live: Query<&mut Text, (With<LiveTrickText>, Without<PopupLine>)>, mut lines: Query<(&PopupLine, &mut Text, &mut Flash), Without<LiveTrickText>>) {
    let Ok(mut live) = live.single_mut() else { return };
    let mut airborne = false;
    for TrickMessage(event) in messages.read() {
        match event {
            TrickEvent::Air { tricks, base } => {
                airborne = true;
                live.0 = format!("{}  {}", trick_line(tricks), thousands(f64::from(*base)));
            }
            TrickEvent::Landed { tricks, grade, total, .. } => {
                let colour = grade_colour(*grade);
                set_popup(&mut lines, &format!("{} ×{}", grade.label(), grade.multiplier()), &trick_line(tricks), &format!("+{}", thousands(f64::from(*total))), colour);
            }
            TrickEvent::Bailed { tricks, lost } => {
                set_popup(&mut lines, Grade::Bail.label(), &trick_line(tricks), &format!("combo lost {}", thousands(f64::from(*lost))), RED);
            }
            TrickEvent::Banked(combo) if combo.value > 0 => {
                let chain = if combo.trick_count > 1 { format!("{}-trick combo {}  ", combo.trick_count, format_multiplier(combo.multiplier)) } else { String::new() };
                set_line(&mut lines, &PopupLine::Bank, format!("{chain}+{}", thousands(f64::from(combo.value))), GOLD, BANK_SECONDS);
            }
            TrickEvent::Banked(_) => {}
        }
    }
    if !airborne {
        live.0.clear();
    }
}

fn show_arrivals(mut teleports: MessageReader<TeleportTo>, mut lines: Query<(&PopupLine, &mut Text, &mut Flash), Without<LiveTrickText>>) {
    if let Some(arrival) = teleports.read().last() {
        set_line(&mut lines, &PopupLine::Bank, arrival.name.to_uppercase(), Color::WHITE, POPUP_SECONDS);
    }
}

fn set_popup(lines: &mut Query<(&PopupLine, &mut Text, &mut Flash), Without<LiveTrickText>>, grade: &str, tricks: &str, points: &str, colour: Color) {
    set_line(lines, &PopupLine::Grade, grade.to_string(), colour, POPUP_SECONDS);
    set_line(lines, &PopupLine::Tricks, tricks.to_string(), Color::WHITE, POPUP_SECONDS);
    set_line(lines, &PopupLine::Points, points.to_string(), colour, POPUP_SECONDS);
}

fn set_line(lines: &mut Query<(&PopupLine, &mut Text, &mut Flash), Without<LiveTrickText>>, which: &PopupLine, value: String, color: Color, seconds: f32) {
    for (line, mut text, mut flash) in lines.iter_mut() {
        if std::mem::discriminant(line) == std::mem::discriminant(which) {
            text.0 = value.clone();
            *flash = Flash { remaining: seconds, color };
        }
    }
}

fn fade_flashes(time: Res<Time>, mut flashes: Query<(&mut Flash, &mut TextColor)>) {
    for (mut flash, mut colour) in &mut flashes {
        flash.remaining = (flash.remaining - time.delta_secs()).max(0.0);
        colour.0 = flash.color.with_alpha((flash.remaining / FADE_SECONDS).min(1.0));
    }
}

fn grade_colour(grade: Grade) -> Color {
    match grade {
        Grade::Perfect => GOLD,
        Grade::Clean => GREEN,
        Grade::Sketchy => TAN,
        Grade::Bail => RED,
    }
}

fn trick_line(tricks: &[Trick]) -> String {
    tricks.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(" + ")
}

fn format_multiplier(m: f64) -> String {
    format!("×{}", (m * 100.0).round() / 100.0)
}

fn format_lat(lat: f64) -> String {
    format!("{:.2}°{}", lat.abs(), if lat >= 0.0 { 'N' } else { 'S' })
}

fn format_lon(lon: f64) -> String {
    let wrapped = (lon + 180.0).rem_euclid(360.0) - 180.0;
    format!("{:.2}°{}", wrapped.abs(), if wrapped >= 0.0 { 'E' } else { 'W' })
}

/// 1234567.8 -> "1,234,568"
fn thousands(value: f64) -> String {
    let rounded = value.round() as i64;
    let digits = rounded.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if rounded < 0 { format!("-{out}") } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_thousands() {
        assert_eq!(thousands(0.0), "0");
        assert_eq!(thousands(999.4), "999");
        assert_eq!(thousands(1234567.8), "1,234,568");
        assert_eq!(thousands(-4312.0), "-4,312");
    }

    #[test]
    fn wraps_longitude_into_east_and_west() {
        assert_eq!(format_lon(-133.8), "133.80°W");
        assert_eq!(format_lon(226.2), "133.80°W");
        assert_eq!(format_lat(-5.4), "5.40°S");
    }
}
