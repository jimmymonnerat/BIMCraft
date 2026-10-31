#!/bin/sh
# Prepara uma máquina nova para transcrever os vídeos (uma vez; cerca de 1 GB): os pacotes Python e, em
# ~/.cache/hidrosan-turbo, o Whisper large-v3-turbo em ONNX (sherpa-onnx), o Silero VAD e o mel_filters.npz do openai-whisper.
# O que já estiver lá não é baixado de novo.
set -eu
dest="${1:-$HOME/.cache/hidrosan-turbo}"
release="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models"
python3 -m pip install -q --break-system-packages numpy onnxruntime sherpa-onnx==2.0.1 tiktoken pillow
mkdir -p "$dest/assets"
cd "$dest"
if [ ! -f sherpa-onnx-whisper-turbo/turbo-decoder.int8.onnx ]; then
    curl -sSL --fail "$release/sherpa-onnx-whisper-turbo.tar.bz2" | tar -xj
fi
if [ ! -f silero_vad.onnx ]; then
    curl -sSL --fail -o silero_vad.onnx "$release/silero_vad.onnx"
fi
if [ ! -f assets/mel_filters.npz ]; then
    python3 -m pip download -q --no-deps --no-binary :all: -d . openai-whisper==20250625
    tar -xzf openai_whisper-20250625.tar.gz openai_whisper-20250625/whisper/assets/mel_filters.npz
    mv openai_whisper-20250625/whisper/assets/mel_filters.npz assets/
    rm -rf openai_whisper-20250625 openai_whisper-20250625.tar.gz
fi
sha256sum -c --quiet <<'SUMS'
b02dcdf54f348741e93fe732b67d933c8dcb6735655f710640143081db38878b  sherpa-onnx-whisper-turbo/turbo-encoder.int8.onnx
20accd02388482eb3a46bd615631adfdc85e1eb2c7db9ea3f02a40ffe6b81547  sherpa-onnx-whisper-turbo/turbo-decoder.int8.onnx
b34b360dbb493e781e479794586d661700670d65564001f23024971d1f2fa126  sherpa-onnx-whisper-turbo/turbo-tokens.txt
9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6  silero_vad.onnx
7450ae70723a5ef9d341e3cee628c7cb0177f36ce42c44b7ed2bf3325f0f6d4c  assets/mel_filters.npz
SUMS
echo "pronto (conferido por SHA-256): $dest"
