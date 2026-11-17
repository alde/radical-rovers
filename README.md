# Radical Rovers

Rover stunt sandbox on the real Moon and Mars, at absurd speed. Rust and
[Bevy](https://bevyengine.org).

Real NASA topography is shrunk to scale so you can cross a planet in minutes
rather than days: Mars at 1:50 (~430 km around), the Moon at 1:25 (~440 km
around). Procedural hills, bumps and craters are layered on top for driving
detail.

## Run

    tools/fetch-dem.sh 4         # once: NASA elevation data, ~2 MB per planet
    cargo run --release
    cargo run --release -- --planet moon --site 3

`--site` picks a landmark (0-based, see `src/planet.rs`). Without the
elevation data the planets are procedural stand-ins.

`--screenshot out.png` saves a frame once the terrain has streamed in, then
quits.

## Controls

| Key | Action |
|---|---|
| W / S | drive, brake, reverse |
| A / D | steer |
| Shift | boost (rockets fire in the air) |
| Space | handbrake |
| ↑ ↓ / ← → / Q E | in the air: pitch, roll, yaw (hands off = auto-level) |
| R | reset upright |
| G | gravity ×1 / ×2 / ×4 |
| 1-9 | teleport to a landmark |

## Tricks

Flips, barrel rolls, spins (in 180s), big air and height score points. The
landing grade multiplies them: PERFECT ×3, CLEAN ×2, SKETCHY ×1, BAIL ×0.
The grade depends on how well the rover lines up with the ground, whether
it points where it's going, and whether it stopped rotating before
touchdown.

Land and do another trick within 2.5 s to chain a combo. Each rotation
trick multiplies the chain by 1.5; hang time and height add points but not
multiplier. The chain banks into your score when the window runs out. A
bail loses the lot.

## Tests

    cargo test

Physics, terrain, scoring and combos are plain Rust with no Bevy systems,
so they test headless.

## Real elevation data

    tools/fetch-dem.sh 4     # ~2 MB per planet, ~15 km/pixel (Mars)
    tools/fetch-dem.sh 16    # ~33 MB per planet, ~3.7 km/pixel (Mars)

Sources: MGS MOLA MEGDR (Mars) and LRO LOLA LDEM (Moon) from the NASA PDS
Geosciences Node, converted to `data/<planet>.bin` + `.json`.

## Layout

- `src/terrain/` – DEM loading, height function (shared by physics and
  rendering), quadtree LOD tiles built on background threads
- `src/rover/` – rigid-body physics (f64), tuning, model
- `src/tricks/` – trick detection, scoring, combo chains
- `src/world.rs` – floating origin: gameplay in f64, rendering re-centred
  near the rover so f32 stays precise ~200 km out
- `src/planet.rs` – gravity, scale, palette, landmarks per planet
