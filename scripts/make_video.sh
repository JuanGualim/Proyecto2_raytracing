#!/usr/bin/env bash
# Arma el video a partir de los frames renderizados en out/frames/.
# Uso: scripts/make_video.sh [salida.mp4]
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="${1:-diorama.mp4}"
FPS="${FPS:-24}"

if ! command -v ffmpeg >/dev/null 2>&1; then
    echo "error: se necesita ffmpeg (herramienta externa, no forma parte del código)" >&2
    exit 1
fi
if [ ! -f out/frames/frame_0000.bmp ]; then
    echo "error: no hay frames; primero ejecuta: cargo run --release -- --preset final" >&2
    exit 1
fi

ffmpeg -y -framerate "$FPS" -i out/frames/frame_%04d.bmp -pix_fmt yuv420p "$OUT"
echo "Video escrito en $OUT"
