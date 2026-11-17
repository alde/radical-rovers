# Planet Rover Madness

Sandbox rover driving on the Moon and Mars. Real topography (once downloaded),
shrunk to scale so you can cross a planet in minutes rather than days:
Mars at 1:50 (~430 km around), the Moon at 1:25 (~440 km around).
Procedural hills, bumps and craters are layered on top for driving detail.

## Run

    python3 -m http.server 8642
    open http://localhost:8642/?planet=mars   # or ?planet=moon

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
- `src/planets.js` – gravity, scale, palette, landmarks per planet
