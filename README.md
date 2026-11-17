# Planet Rover Madness

Sandbox rover driving on the Moon and Mars. Real topography (once downloaded),
shrunk to scale so you can cross a planet in minutes rather than days:
Mars at 1:50 (~430 km around), the Moon at 1:25 (~440 km around).
Procedural hills, bumps and craters are layered on top for driving detail.

## Run

    python3 -m http.server 8642
    open http://localhost:8642/?planet=mars   # or ?planet=moon

## Tricks

Flips, barrel rolls, spins (in 180s), big air and height score points.
The landing grade multiplies them: PERFECT ×3, CLEAN ×2, SKETCHY ×1, BAIL
×0. Grade depends on how well the rover lines up with the ground, whether
it is pointing where it is going, and whether it stopped rotating before
touchdown.

Land and take off again within 2.5 s of ground time to chain a combo. Each
rotation trick (flip, roll, spin) multiplies the chain by 1.5; hang time and
height add points but not multiplier. The chain banks into your score when
the window runs out. A bail loses the lot.

Rules live in `src/tricks/scoring.js` and `src/tricks/combo-chain.js`.

## Tests

Open http://localhost:8642/tests/ with the server running. The title reads
PASS or FAIL.

## Real elevation data

Without data the planets are procedural stand-ins. To use NASA's maps:

    tools/fetch-dem.sh 4     # ~2 MB per planet, ~15 km/pixel (Mars)
    tools/fetch-dem.sh 16    # ~33 MB per planet, ~3.7 km/pixel (Mars)

Sources: MGS MOLA MEGDR (Mars) and LRO LOLA LDEM (Moon) from the NASA PDS
Geosciences Node. Converted to `data/<planet>.bin` + `.json`.

## Layout

- `src/terrain/` – DEM loading, height function (shared by physics and
  rendering), quadtree LOD chunk streaming
- `src/rover/` – rigid-body physics, spec/tuning, model
- `src/tricks/` – trick detection, scoring, popups and sounds
- `src/planets.js` – gravity, scale, palette, landmarks per planet
