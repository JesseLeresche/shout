#!/usr/bin/env bash
# Download the STT models shout needs into ./models (gitignored).
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p models
cd models

PARAKEET=sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8
if [ -d "$PARAKEET" ]; then
  echo "$PARAKEET already present"
else
  echo "Downloading $PARAKEET (~480MB)…"
  curl -L -o "$PARAKEET.tar.bz2" \
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/$PARAKEET.tar.bz2"
  tar xjf "$PARAKEET.tar.bz2"
  rm "$PARAKEET.tar.bz2"
fi
