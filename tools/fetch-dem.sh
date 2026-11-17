#!/usr/bin/env bash
# Download real elevation data from NASA PDS and convert it for the game.
# Usage: tools/fetch-dem.sh [4|16]   (pixels per degree; 4 = ~2 MB each, 16 = ~33 MB each)
set -euo pipefail
cd "$(dirname "$0")/.."

RES="${1:-4}"
PDS=https://pds-geosciences.wustl.edu
case "$RES" in
  4)  MARS=$PDS/mgs/mgs-m-mola-5-megdr-l3-v1/mgsl_300x/meg004/megt90n000cb
      MOON=$PDS/lro/lro-l-lola-3-rdr-v1/lrolol_1xxx/data/lola_gdr/cylindrical/img/ldem_4 ;;
  16) MARS=$PDS/mgs/mgs-m-mola-5-megdr-l3-v1/mgsl_300x/meg016/megt90n000eb
      MOON=$PDS/lro/lro-l-lola-3-rdr-v1/lrolol_1xxx/data/lola_gdr/cylindrical/img/ldem_16 ;;
  *)  echo "Resolution must be 4 or 16" >&2; exit 1 ;;
esac

mkdir -p data/raw
for pair in "mars $MARS" "moon $MOON"; do
  read -r planet base <<<"$pair"
  name=$(basename "$base")
  for ext in lbl img; do
    curl -fL --progress-bar -o "data/raw/$name.$ext" "$base.$ext"
  done
  python3 -I tools/convert_dem.py "data/raw/$name.lbl" "data/raw/$name.img" "$planet" data
done
