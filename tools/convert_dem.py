#!/usr/bin/env python3
"""Convert a NASA PDS3 elevation image (MOLA MEGDR / LOLA LDEM) for the game.

Writes data/<planet>.bin (little-endian int16, raw DN) and data/<planet>.json
(width, height, ppd, lon0, scale). The browser multiplies DN by `scale` to get
metres. OFFSET in LOLA labels is the reference radius, so it is ignored: we
want height above datum, not distance from the centre.

Usage: convert_dem.py <label.lbl> <image.img> <planet-id> <out-dir>
"""
import json
import re
import sys
from array import array
from pathlib import Path

KEY_VALUE = re.compile(r"^\s*([A-Z_^]+)\s*=\s*(.+?)\s*$")


def parse_label(text):
    fields = {}
    for line in text.splitlines():
        match = KEY_VALUE.match(line)
        if match and match.group(1) not in fields:
            fields[match.group(1)] = re.sub(r"<.*?>", "", match.group(2)).strip().strip('"')
    return fields


def read_samples(image_path, fields):
    if fields.get("SAMPLE_BITS", "16") != "16":
        raise SystemExit(f"Only 16-bit samples supported, got {fields['SAMPLE_BITS']}")
    samples = array("h")
    samples.frombytes(Path(image_path).read_bytes())
    big_endian = fields["SAMPLE_TYPE"].startswith("MSB")
    if big_endian != (sys.byteorder == "big"):
        samples.byteswap()
    expected = int(fields["LINES"]) * int(fields["LINE_SAMPLES"])
    if len(samples) != expected:
        raise SystemExit(f"Expected {expected} samples, file has {len(samples)}")
    return samples


def main(label_path, image_path, planet, out_dir):
    fields = parse_label(Path(label_path).read_text(errors="replace"))
    samples = read_samples(image_path, fields)
    if sys.byteorder == "big":
        samples.byteswap()  # output is always little-endian, as the browser expects
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / f"{planet}.bin").write_bytes(samples.tobytes())
    meta = {
        "width": int(fields["LINE_SAMPLES"]),
        "height": int(fields["LINES"]),
        "ppd": float(fields["MAP_RESOLUTION"]),
        "lon0": float(fields.get("WESTERNMOST_LONGITUDE", 0)),
        "scale": float(fields.get("SCALING_FACTOR", 1)),
    }
    (out / f"{planet}.json").write_text(json.dumps(meta, indent=2))
    print(f"{planet}: {meta}")


if __name__ == "__main__":
    if len(sys.argv) != 5:
        raise SystemExit(__doc__)
    main(*sys.argv[1:])
