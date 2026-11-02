#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo xtask bundle musa-plugin --release

mkdir -p ~/.clap ~/.vst3
cp "target/bundled/Harmony Musa.clap" ~/.clap/
rm -rf ~/.vst3/"Harmony Musa.vst3"
cp -r "target/bundled/Harmony Musa.vst3" ~/.vst3/
echo "Instalado en ~/.clap y ~/.vst3"
