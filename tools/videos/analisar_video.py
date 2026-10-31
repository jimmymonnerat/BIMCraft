#!/usr/bin/env python3
"""Turn a screen recording of a design session into material an agent can study.

For each video it writes, in its own folder:
  <name>.txt          the narration, transcribed (Whisper small, Portuguese), one line per phrase with [mm:ss]
  <name>.srt          the same as subtitles, to watch the video with them
  quadros/            key frames: one after each change of the screen (a dialog opening, a tab switched, a view zoomed),
                      taken a moment later so the screen has settled, and one at least every --intervalo-max seconds
  folhas/             contact sheets of the key frames, six to a sheet, each labelled with its time
  roteiro.md          the timeline: each key frame with what was said while it was on screen
  dados.json          all of it as data (segments, key frames, their scene scores)

It only reads the video. The speech model is the faster-whisper conversion of OpenAI Whisper small (MIT), published on npm in
four parts; it is fetched once, checked against a pinned SHA-256 and kept in --modelo.

Usage:
  python3 tools/videos/analisar_video.py VIDEO [VIDEO ...] --saida DIR
      [--vocabulario tools/videos/vocabulario.txt] [--correcoes tools/videos/correcoes.txt] [--sem-transcricao]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import wave
from dataclasses import dataclass, asdict
from pathlib import Path

HERE = Path(__file__).resolve().parent

# The model: Whisper small as CTranslate2 int8, in four npm packages (npm versions never change once published).
MODEL_PARTS = [
    f"https://registry.npmjs.org/@skillninja/shortmovie-telop-model-p{i}/-/shortmovie-telop-model-p{i}-1.0.0.tgz" for i in range(4)
]
MODEL_SHA256 = "ca732cb1379efc49417d028465f543d83a39de2c61f3027678de103dc404ce73"
MODEL_FILES = {"config.json", "tokenizer.json", "vocabulary.json", "preprocessor_config.json", "model.sha256"}


@dataclass
class Segment:
    start: float
    end: float
    text: str


@dataclass
class Frame:
    time: float
    score: float
    file: str


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, check=True, capture_output=True, text=True, **kw)


def clock(t: float) -> str:
    t = max(0.0, t)
    return f"{int(t // 60):02d}:{int(t % 60):02d}"


def srt_clock(t: float) -> str:
    t = max(0.0, t)
    ms = int(round(t * 1000))
    return f"{ms // 3600000:02d}:{ms // 60000 % 60:02d}:{ms // 1000 % 60:02d},{ms % 1000:03d}"


def has_audio(video: Path) -> bool:
    out = run(["ffprobe", "-v", "error", "-select_streams", "a", "-show_entries", "stream=index", "-of", "csv=p=0", str(video)])
    return bool(out.stdout.strip())


def duration(video: Path) -> float:
    out = run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(video)]).stdout
    return float(out.strip() or 0)


# ---------- the model ----------

def ensure_model(folder: Path) -> Path:
    """The speech model in `folder`, fetched and checked the first time."""
    model = folder / "model.bin"
    if model.exists() and sha256(model) == MODEL_SHA256:
        return folder
    folder.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        parts = []
        for i, url in enumerate(MODEL_PARTS):
            tgz = Path(tmp) / f"p{i}.tgz"
            print(f"  modelo: baixando parte {i + 1}/4", file=sys.stderr)
            run(["curl", "-sSfL", "-o", str(tgz), url])
            with tarfile.open(tgz) as t:
                for m in t.getmembers():
                    name = Path(m.name).name
                    if not m.isfile():
                        continue
                    if name == f"model.bin.part{i}" or (i == 0 and name in MODEL_FILES):
                        m.name = name
                        t.extract(m, tmp, filter="data")
            parts.append(Path(tmp) / f"model.bin.part{i}")
        with open(model, "wb") as out:
            for p in parts:
                with open(p, "rb") as src:
                    shutil.copyfileobj(src, out)
        for name in MODEL_FILES - {"model.sha256"}:
            shutil.copy(Path(tmp) / name, folder / name)
    got = sha256(model)
    if got != MODEL_SHA256:
        model.unlink()
        raise SystemExit(f"o modelo baixado não confere (sha256 {got}); nada foi usado")
    return folder


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


# ---------- speech ----------

def words(files: list[Path]) -> str:
    """The vocabulary that steers recognition: one term a line, `#` a comment."""
    terms = []
    for f in files:
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.split("#", 1)[0].strip()
            if line:
                terms.append(line)
    return ", ".join(terms)


def corrections(files: list[Path]) -> list[tuple[re.Pattern, str]]:
    """`wrong => right` a line (whole words), or `re: pattern => replacement`."""
    out = []
    for f in files:
        for line in f.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line or line.startswith("#") or "=>" not in line:
                continue
            left, right = (s.strip() for s in line.split("=>", 1))
            if left.startswith("re:"):
                out.append((re.compile(left[3:].strip()), right))
            else:
                out.append((re.compile(rf"\b{re.escape(left)}\b"), right))
    return out


def transcribe(video: Path, model_dir: Path, prompt: str, fixes) -> list[Segment]:
    import numpy as np
    from faster_whisper import WhisperModel

    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "audio.wav"
        run(["ffmpeg", "-v", "error", "-y", "-i", str(video), "-vn", "-ac", "1", "-ar", "16000", str(wav)])
        with wave.open(str(wav)) as w:
            audio = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float32) / 32768.0
    if audio.size == 0:
        return []
    model = WhisperModel(str(model_dir), device="cpu", compute_type="int8", cpu_threads=os.cpu_count() or 2)
    segs, _ = model.transcribe(audio, language="pt", vad_filter=True, beam_size=5, initial_prompt=prompt or None)
    out = []
    for s in segs:
        text = s.text.strip()
        for pattern, repl in fixes:
            text = pattern.sub(repl, text)
        out.append(Segment(round(s.start, 2), round(s.end, 2), text))
    return out


# ---------- frames ----------

def scene_scores(video: Path) -> list[tuple[float, float]]:
    """(time, how much the picture changed from the frame before), for every frame, at a small size."""
    vf = "scale=320:-2,select='gte(scene,0)',metadata=print:key=lavfi.scene_score"
    p = subprocess.run(["ffmpeg", "-hide_banner", "-i", str(video), "-vf", vf, "-an", "-f", "null", "-"],
                       capture_output=True, text=True)
    out, t = [], None
    for line in p.stderr.splitlines():
        m = re.search(r"pts_time:([0-9.]+)", line)
        if m:
            t = float(m.group(1))
            continue
        m = re.search(r"lavfi\.scene_score=([0-9.]+)", line)
        if m and t is not None:
            out.append((t, float(m.group(1))))
            t = None
    return out


def pick(scores: list[tuple[float, float]], length: float, threshold: float, min_gap: float, max_gap: float,
         settle: float) -> list[tuple[float, float]]:
    """The moments to look at: the picture just after each change above `threshold` (the strongest of any closer than
    `min_gap`), `settle` seconds on so it has stopped moving, the start, and enough between them that no gap is longer than
    `max_gap`."""
    changes: list[tuple[float, float]] = []
    for t, s in scores:
        if s < threshold:
            continue
        if changes and t - changes[-1][0] < min_gap:
            if s > changes[-1][1]:
                changes[-1] = (t, s)
            continue
        changes.append((t, s))
    moments = [(min(0.5, length), 0.0)] + [(min(t + settle, max(length - 0.1, 0.0)), s) for t, s in changes]
    moments.sort()
    filled: list[tuple[float, float]] = []
    for t, s in moments:
        if filled and t - filled[-1][0] < min_gap:
            continue
        while filled and t - filled[-1][0] > max_gap:
            filled.append((filled[-1][0] + max_gap, 0.0))
        filled.append((t, s))
    while filled and length - filled[-1][0] > max_gap:
        filled.append((filled[-1][0] + max_gap, 0.0))
    return filled


def grab(video: Path, moments: list[tuple[float, float]], folder: Path) -> list[Frame]:
    folder.mkdir(parents=True, exist_ok=True)
    frames = []
    for n, (t, s) in enumerate(moments, 1):
        name = f"q{n:03d}_{clock(t).replace(':', 'm')}s.jpg"
        run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.2f}", "-i", str(video), "-frames:v", "1", "-q:v", "2",
             str(folder / name)])
        frames.append(Frame(round(t, 2), round(s, 3), name))
    return frames


def sheets(frames: list[Frame], quadros: Path, folder: Path, cols: int = 2, rows: int = 3, width: int = 768) -> list[str]:
    from PIL import Image, ImageDraw, ImageFont

    folder.mkdir(parents=True, exist_ok=True)
    try:
        font = ImageFont.truetype("DejaVuSans-Bold.ttf", 26)
    except OSError:
        font = ImageFont.load_default()
    made = []
    per = cols * rows
    for k in range(0, len(frames), per):
        group = frames[k:k + per]
        tiles = []
        for i, f in enumerate(group):
            im = Image.open(quadros / f.file).convert("RGB")
            im = im.resize((width, int(im.height * width / im.width)))
            d = ImageDraw.Draw(im)
            label = f"{clock(f.time)}  #{k + i + 1}"
            box = d.textbbox((8, 6), label, font=font)
            d.rectangle((box[0] - 4, box[1] - 3, box[2] + 4, box[3] + 3), fill="black")
            d.text((8, 6), label, font=font, fill="yellow")
            tiles.append(im)
        h = max(t.height for t in tiles)
        sheet = Image.new("RGB", (cols * width, rows * h), "black")
        for i, t in enumerate(tiles):
            sheet.paste(t, ((i % cols) * width, (i // cols) * h))
        name = f"folha_{k // per + 1:02d}.jpg"
        sheet.save(folder / name, quality=85)
        made.append(name)
    return made


# ---------- the timeline ----------

def timeline(name: str, length: float, frames: list[Frame], segments: list[Segment]) -> str:
    lines = [f"# Roteiro: {name}", "", f"Duração {clock(length)}; {len(frames)} quadros-chave; {len(segments)} falas.",
             "Cada quadro com o que foi dito enquanto ele estava na tela (até o quadro seguinte).", "",
             "| # | tempo | quadro | fala |", "|---|---|---|---|"]
    for i, f in enumerate(frames):
        end = frames[i + 1].time if i + 1 < len(frames) else length + 1
        said = " ".join(s.text for s in segments if f.time - 0.01 <= s.start < end or (i == 0 and s.start < f.time))
        said = said.replace("|", "/")
        lines.append(f"| {i + 1} | {clock(f.time)} | {f.file} | {said} |")
    return "\n".join(lines) + "\n"


def slug(name: str) -> str:
    return re.sub(r"\s+", " ", name).strip()


def analyse(video: Path, out_root: Path, args, model_dir: Path | None, prompt: str, fixes) -> Path:
    name = slug(video.stem)
    out = out_root / name
    out.mkdir(parents=True, exist_ok=True)
    length = duration(video)
    print(f"{name}: {clock(length)}", file=sys.stderr)
    segments: list[Segment] = []
    silent = not has_audio(video)
    if silent:
        print("  sem faixa de áudio: nada a transcrever", file=sys.stderr)
    if model_dir is not None and not silent:
        segments = transcribe(video, model_dir, prompt, fixes)
        (out / f"{name}.txt").write_text(
            "".join(f"[{clock(s.start)}] {s.text}\n" for s in segments), encoding="utf-8")
        (out / f"{name}.srt").write_text(
            "".join(f"{i}\n{srt_clock(s.start)} --> {srt_clock(s.end)}\n{s.text}\n\n" for i, s in enumerate(segments, 1)),
            encoding="utf-8")
    moments = pick(scene_scores(video), length, args.limiar, args.intervalo_min, args.intervalo_max, args.assentar)
    frames = grab(video, moments, out / "quadros")
    folhas = sheets(frames, out / "quadros", out / "folhas")
    text = timeline(name, length, frames, segments)
    if silent:
        text = text.replace("\n\n|", "\n\n**O vídeo não tem faixa de áudio: não há fala a transcrever.**\n\n|", 1)
    (out / "roteiro.md").write_text(text, encoding="utf-8")
    (out / "dados.json").write_text(json.dumps({
        "video": video.name, "duracao": length, "sem_audio": silent, "segmentos": [asdict(s) for s in segments],
        "quadros": [asdict(f) for f in frames], "folhas": folhas,
    }, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"  {len(segments)} falas, {len(frames)} quadros, {len(folhas)} folhas -> {out}", file=sys.stderr)
    return out


def main() -> None:
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("videos", nargs="+", type=Path)
    a.add_argument("--saida", type=Path, required=True, help="pasta onde cada vídeo ganha a sua")
    a.add_argument("--modelo", type=Path, default=Path.home() / ".cache" / "hidrosan-whisper")
    a.add_argument("--vocabulario", type=Path, action="append", default=None,
                   help="termos que orientam o reconhecimento (padrão: tools/videos/vocabulario.txt)")
    a.add_argument("--correcoes", type=Path, action="append", default=None,
                   help="trocas aplicadas ao texto (padrão: tools/videos/correcoes.txt)")
    a.add_argument("--sem-transcricao", action="store_true")
    a.add_argument("--limiar", type=float, default=0.04, help="quanto a tela precisa mudar para virar quadro-chave")
    a.add_argument("--intervalo-min", type=float, default=2.0)
    a.add_argument("--intervalo-max", type=float, default=15.0)
    a.add_argument("--assentar", type=float, default=0.8, help="segundos depois da mudança, para a tela parar")
    args = a.parse_args()
    vocab = args.vocabulario or [HERE / "vocabulario.txt"]
    fixes_files = args.correcoes or [HERE / "correcoes.txt"]
    model_dir = None if args.sem_transcricao else ensure_model(args.modelo)
    prompt, fixes = words(vocab), corrections(fixes_files)
    for v in args.videos:
        analyse(v, args.saida, args, model_dir, prompt, fixes)


if __name__ == "__main__":
    main()
