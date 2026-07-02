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

# Ghost-mode models (VAD + Whisper + diarization); pass --ghost to fetch them.
if [ "${1:-}" = "--ghost" ]; then
  if [ ! -f silero_vad.onnx ]; then
    echo "Downloading silero_vad.onnx…"
    curl -L -O "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx"
  fi

  SEG=sherpa-onnx-pyannote-segmentation-3-0
  if [ ! -d "$SEG" ]; then
    echo "Downloading $SEG…"
    curl -L -o "$SEG.tar.bz2" \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/$SEG.tar.bz2"
    tar xjf "$SEG.tar.bz2"
    rm "$SEG.tar.bz2"
  fi

  EMB=3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx
  if [ ! -f "$EMB" ]; then
    echo "Downloading $EMB…"
    curl -L -O \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/$EMB"
  fi

  if [ ! -f ggml-large-v3.bin ]; then
    echo "Downloading Whisper Large V3 (~3.1GB)…"
    curl -L -o ggml-large-v3.bin \
      "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin"
  fi
fi
