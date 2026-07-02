#!/usr/bin/env bash
# Download the STT models shout needs.
#
# Inside a shout git checkout, models land in ./models (gitignored) so dev
# builds pick them up automatically. Run standalone (e.g. downloaded on its
# own, no repo clone) they land in ~/.config/shout/models instead, which is
# where an installed shout.app looks for them. Override with SHOUT_MODELS_DIR.
set -euo pipefail
script_dir="$(cd "$(dirname "$0")" && pwd)"
if [ -n "${SHOUT_MODELS_DIR:-}" ]; then
  dest="$SHOUT_MODELS_DIR"
elif [ -d "$script_dir/../.git" ]; then
  dest="$script_dir/../models"
else
  dest="$HOME/.config/shout/models"
fi
mkdir -p "$dest"
echo "Downloading models into $dest"
cd "$dest"

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

# Ghost-mode models (VAD + Whisper + diarization); pass --ghost to fetch them.
if [ "${1:-}" = "--ghost" ]; then
  if [ ! -f silero_vad.onnx ]; then
    echo "Downloading silero_vad.onnx…"
    curl -L -O "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx"
  fi

  SEG=sherpa-onnx-pyannote-segmentation-3-0
  if [ ! -d "$SEG" ]; then
    echo "Downloading ${SEG}..."
    curl -L -o "$SEG.tar.bz2" \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/$SEG.tar.bz2"
    tar xjf "$SEG.tar.bz2"
    rm "$SEG.tar.bz2"
  fi

  EMB=3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx
  if [ ! -f "$EMB" ]; then
    echo "Downloading ${EMB}..."
    curl -L -O \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/$EMB"
  fi

  if [ ! -f ggml-large-v3.bin ]; then
    echo "Downloading Whisper Large V3 (~3.1GB)…"
    curl -L -o ggml-large-v3.bin \
      "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"
  fi
fi
