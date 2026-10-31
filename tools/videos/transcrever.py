#!/usr/bin/env python3
"""Legendas, transcrição e roteiro de um vídeo do QiBuilder, com o Whisper large-v3-turbo.

Usa a escolha de quadros-chave, as folhas e o roteiro de tools/videos/analisar_video.py; troca a
fala: o Whisper "turbo" decodificado como o original (whisper_onnx.py), com o vocabulário e a fala anterior como contexto,
os silêncios longos cortados antes, e as legendas em até duas linhas de 42 caracteres.

A fala bruta (o que o modelo devolveu, nos tempos da fala juntada, e o mapa de volta ao vídeo) fica em dados.json, para
refazer as legendas em segundos (--so-legendas) depois de acrescentar correções.

Uso:
  python3 -I transcrever.py VIDEO [VIDEO ...] --saida DIR [--modelo DIR] [--assets DIR]
      [--vocabulario-frase ARQ] [--correcoes ARQ ...] [--sem-quadros]
  python3 -I transcrever.py --so-legendas --saida DIR [--correcoes ARQ ...] NOME [NOME ...]
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
import tempfile
import time
import unicodedata
import wave
from dataclasses import asdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL = HERE / "analisar_video.py"
MODELS = Path.home() / ".cache" / "hidrosan-turbo"


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod  # as dataclasses do módulo procuram por ele aqui
    spec.loader.exec_module(mod)
    return mod


av = _load("analisar_video", TOOL)
W = _load("whisper_onnx", HERE / "whisper_onnx.py")

RATE = 16000
LINE = 42  # caracteres por linha de legenda
CUE_MAX_CHARS = 74  # duas linhas de 42, com folga para quebrar entre palavras
CUE_MAX_SECONDS = 7.0
CUE_MIN_SECONDS = 1.0
MODEL_NAME = "Whisper large-v3-turbo int8 (ONNX do sherpa-onnx), decodificação própria com contexto e tempos; Silero VAD; pt"
# Frases que o Whisper inventa em silêncio ou ruído (em português).
HALLUCINATIONS = [
    r"legendas? (pela|por) comunidade amara\.org",
    r"obrigad[oa] por assistir",
    r"inscreva-se no canal",
    r"^\W*$",
]


def load_audio(video: Path):
    import numpy as np

    with tempfile.TemporaryDirectory() as tmp:
        wav = Path(tmp) / "audio.wav"
        av.run(["ffmpeg", "-v", "error", "-y", "-i", str(video), "-vn", "-ac", "1", "-ar", str(RATE), str(wav)])
        with wave.open(str(wav)) as w:
            data = w.readframes(w.getnframes())
    return np.frombuffer(data, dtype=np.int16).astype(np.float32) / 32768.0


# ---------- texto ----------

def clean(text: str, fixes) -> str:
    text = re.sub(r"\s+", " ", text).strip()
    for pattern, repl in fixes:
        text = pattern.sub(repl, text)
    return re.sub(r"\s+", " ", text).strip()


def norm(text: str) -> str:
    t = unicodedata.normalize("NFD", text.lower())
    t = "".join(c for c in t if unicodedata.category(c) != "Mn")
    return re.sub(r"[^a-z0-9]+", " ", t).strip()


def is_noise(text: str) -> bool:
    low = text.lower().strip()
    return any(re.search(p, low) for p in HALLUCINATIONS)


def sentences(text: str) -> list[str]:
    return [s for s in re.split(r"(?<=[.?!…])\s+", text.strip()) if s]


def reading_time(text: str) -> float:
    """Quanto uma legenda fica na tela: 1,2 s mais 14 caracteres por segundo, até 8 s."""
    return min(8.0, 1.2 + len(text) / 14)


def split_text(text: str, limit: int = CUE_MAX_CHARS) -> list[str]:
    """O texto em pedaços de até `limit` caracteres: nas frases, depois nas vírgulas, depois entre palavras."""
    if len(text) <= limit:
        return [text]
    for sep in (r"(?<=[.?!…])\s+", r"(?<=[,;:])\s+"):
        parts = [p for p in re.split(sep, text) if p]
        if len(parts) > 1:
            out, cur = [], ""
            for p in parts:
                if cur and len(cur) + 1 + len(p) > limit:
                    out.append(cur)
                    cur = p
                else:
                    cur = f"{cur} {p}".strip()
            if cur:
                out.append(cur)
            if all(len(o) <= limit for o in out):
                return out
            return [piece for o in out for piece in split_text(o, limit)]
    words = text.split()
    out, cur = [], ""
    n = -(-len(text) // limit)
    target = len(text) / n
    for w in words:
        if cur and (len(cur) + 1 + len(w) > limit or len(cur) >= target):
            out.append(cur)
            cur = w
        else:
            cur = f"{cur} {w}".strip()
    if cur:
        out.append(cur)
    return out


def wrap(text: str) -> str:
    """Uma legenda em até duas linhas de no máximo 42 caracteres, equilibradas."""
    if len(text) <= LINE:
        return text
    words = text.split()
    best = None
    for i in range(1, len(words)):
        a, b = " ".join(words[:i]), " ".join(words[i:])
        if len(a) <= LINE and len(b) <= LINE:
            score = abs(len(a) - len(b))
            if best is None or score < best[0]:
                best = (score, a, b)
    if best is None:  # nenhuma quebra deixa as duas linhas com 42: a mais equilibrada
        i = min(range(1, len(words)), key=lambda k: max(len(" ".join(words[:k])), len(" ".join(words[k:]))), default=None)
        if i is None:
            return text
        return " ".join(words[:i]) + "\n" + " ".join(words[i:])
    return f"{best[1]}\n{best[2]}"


def cues_for(start: float, end: float, text: str) -> list[tuple[float, float, str]]:
    """Um trecho de fala em legendas: cada pedaço com uma parte do tempo proporcional ao seu tamanho."""
    pieces = split_text(text)
    if len(pieces) == 1 and end - start > CUE_MAX_SECONDS and len(text) > LINE:
        pieces = split_text(text, max(LINE, len(text) // 2 + 1))
    total = sum(len(p) for p in pieces) or 1
    out, t = [], start
    for p in pieces:
        d = (end - start) * len(p) / total
        out.append((t, t + d, p))
        t += d
    return out


def split_at(start: float, end: float, text: str, bounds: list[float]) -> list[tuple[float, float, str]]:
    """Um trecho que atravessa um silêncio cortado, em partes, uma de cada lado, com as palavras pelo tempo de fala."""
    cuts = [b for b in bounds if start + 0.05 < b < end - 0.05]
    if not cuts:
        return [(start, end, text)]
    pts = [start] + cuts + [end]
    words = text.split()
    total_t = end - start
    total_c = sum(len(w) + 1 for w in words)
    out, i, acc = [], 0, 0
    for k in range(len(pts) - 1):
        goal = total_c * (pts[k + 1] - start) / total_t
        part = []
        while i < len(words) and (acc + len(words[i]) / 2 <= goal or k == len(pts) - 2):
            part.append(words[i])
            acc += len(words[i]) + 1
            i += 1
        if part:
            out.append((pts[k], pts[k + 1], " ".join(part)))
    return out


# ---------- da fala bruta às legendas ----------

def back_fn(table: list):
    def back(t: float) -> float:
        for cs, orig, ln in table:
            if t <= cs + ln + 1e-6:
                return orig + max(0.0, t - cs)
        cs, orig, _ = table[-1]
        return orig + (t - cs)
    return back


def drop_repeated_blocks(kept: list, report: list, longest: int = 6, gap: float = 20.0) -> list:
    """Tira um bloco de frases repetido logo em seguida (A B A B → A B), o eco que o Whisper deixa na emenda das janelas."""
    changed = True
    while changed:
        changed = False
        for i in range(1, len(kept)):
            for k in range(min(longest, i, len(kept) - i), 0, -1):
                first, second = kept[i - k:i], kept[i:i + k]
                if second[0]["a"] - first[-1]["b"] > gap:
                    continue
                if [norm(x["texto"]) for x in first] == [norm(x["texto"]) for x in second]:
                    for x in second:
                        report[x["r"]]["descartado"] = "repete o bloco anterior"
                    kept = kept[:i] + kept[i + k:]
                    changed = True
                    break
            if changed:
                break
    return kept


def subtitles(raw: dict, fixes, duration: float) -> tuple[list, list]:
    """(legendas, trechos com o que foi descartado e por quê), a partir da fala bruta guardada."""
    table = raw["tabela"]
    if not table:
        return [], []
    back = back_fn(table)
    bounds = [cs for cs, _, _ in table[1:]]
    speech_end = table[-1][1] + table[-1][2]
    kept, report = [], []
    for s in raw["trechos"]:
        text = clean(s["texto_bruto"], fixes)
        a, b = back(s["a"]), back(s["b"])
        why = None
        if is_noise(text):
            why = "ruído ou frase inventada"
        elif a >= min(duration, speech_end) - 0.25:
            why = "depois do fim da fala (inventado)"
        elif kept and norm(text) == norm(kept[-1]["texto"]):
            why = "repete o trecho anterior"
        elif kept:
            last = sentences(kept[-1]["texto"])
            first = sentences(text)
            if last and first and len(norm(last[-1])) >= 8 and norm(first[0]) == norm(last[-1]):
                text = " ".join(first[1:]).strip()  # a mesma frase no fim de uma janela e no começo da outra
                if not text:
                    why = "repete o fim do trecho anterior"
        report.append({"inicio": round(a, 2), "fim": round(min(b, duration), 2), "texto": text,
                       "descartado": why, "logprob_medio_da_janela": s["lp"], "prob_silencio": s["ns"],
                       "compressao": s["cr"], "temperatura": s["t"]})
        if why is None:
            kept.append({"a": s["a"], "b": s["b"], "texto": text, "r": len(report) - 1})
    kept = drop_repeated_blocks(kept, report)
    cues = []
    for s in kept:
        for a, b, part in split_at(s["a"], s["b"], s["texto"], bounds):
            for ca, cb, ct in cues_for(a, b, part):
                cues.append((back(ca), min(back(cb), duration), ct))
    fixed = []
    for i, (a, b, t) in enumerate(cues):
        nxt = cues[i + 1][0] if i + 1 < len(cues) else min(duration, b + CUE_MIN_SECONDS)
        b = min(b, a + reading_time(t))  # some depois do tempo de leitura, sem ficar na tela durante as pausas
        b = max(b, min(a + CUE_MIN_SECONDS, nxt))
        fixed.append(av.Segment(round(a, 2), round(b, 2), t))
    return fixed, report


def raw_speech(video: Path, args, rec) -> dict:
    audio = load_audio(video)
    segs, line = W.transcribe_speech(rec, audio, args.modelo / "silero_vad.onnx", args.frase)
    return {"tabela": [[round(x, 3) for x in r] for r in line.table],
            "trechos": [{"a": round(s.start, 3), "b": round(s.end, 3), "texto_bruto": s.text,
                         "lp": round(s.avg_logprob, 3), "ns": round(s.no_speech_prob, 3),
                         "cr": round(s.compression_ratio, 2), "t": s.temperature} for s in segs]}


# ---------- saída ----------

def write_outputs(out: Path, name: str, data: dict, segments: list, report: list, frames, folhas) -> None:
    if not data["sem_audio"]:
        (out / f"{name}.txt").write_text("".join(f"[{av.clock(s.start)}] {s.text}\n" for s in segments), encoding="utf-8")
        (out / f"{name}.srt").write_text(
            "".join(f"{i}\n{av.srt_clock(s.start)} --> {av.srt_clock(s.end)}\n{wrap(s.text)}\n\n"
                    for i, s in enumerate(segments, 1)), encoding="utf-8")
    text = av.timeline(name, data["duracao"], frames, segments)
    if data["sem_audio"]:
        text = text.replace("\n\n|", "\n\n**O vídeo não tem faixa de áudio: não há fala a transcrever.**\n\n|", 1)
    (out / "roteiro.md").write_text(text, encoding="utf-8")
    data.update({"segmentos": [asdict(s) for s in segments], "trechos_ouvidos": report,
                 "quadros": [asdict(f) for f in frames], "folhas": folhas})
    (out / "dados.json").write_text(json.dumps(data, ensure_ascii=False, indent=1), encoding="utf-8")


def analyse(video: Path, out_root: Path, args, fixes, rec) -> Path:
    name = av.slug(video.stem)
    out = out_root / name
    out.mkdir(parents=True, exist_ok=True)
    length = av.duration(video)
    print(f"{name}: {av.clock(length)}", file=sys.stderr)
    silent = not av.has_audio(video)
    data = {"video": video.name, "duracao": length, "sem_audio": silent, "modelo_de_fala": MODEL_NAME,
            "frase_de_contexto_usada": bool(args.frase)}
    raw, took = {"tabela": [], "trechos": []}, 0.0
    if not silent:
        t0 = time.time()
        raw = raw_speech(video, args, rec)
        took = time.time() - t0
        print(f"  fala: {len(raw['trechos'])} trechos em {took:.0f} s ({took / max(length, 1):.2f}× a duração)",
              file=sys.stderr)
    data["segundos_de_processamento_da_fala"] = round(took)
    data["fala_bruta"] = raw
    kept = out / "dados.json"
    if args.sem_quadros and kept.exists():
        old = json.loads(kept.read_text(encoding="utf-8"))
        frames, folhas = [av.Frame(**f) for f in old["quadros"]], old["folhas"]
    else:
        moments = av.pick(av.scene_scores(video), length, 0.04, 2.0, 15.0, 0.8)
        frames = av.grab(video, moments, out / "quadros")
        folhas = av.sheets(frames, out / "quadros", out / "folhas")
    segments, report = subtitles(raw, fixes, length)
    write_outputs(out, name, data, segments, report, frames, folhas)
    print(f"  {len(segments)} legendas, {len(frames)} quadros, {len(folhas)} folhas -> {out}", file=sys.stderr)
    return out


def redo_subtitles(out: Path, fixes) -> None:
    data = json.loads((out / "dados.json").read_text(encoding="utf-8"))
    frames, folhas = [av.Frame(**f) for f in data["quadros"]], data["folhas"]
    segments, report = subtitles(data["fala_bruta"], fixes, data["duracao"])
    write_outputs(out, out.name, data, segments, report, frames, folhas)
    gone = sum(1 for r in report if r["descartado"])
    print(f"{out.name}: {len(segments)} legendas; {gone} trechos descartados", file=sys.stderr)


def main() -> None:
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("videos", nargs="+")
    a.add_argument("--saida", type=Path, required=True)
    a.add_argument("--modelo", type=Path, default=MODELS, help="a pasta que preparar.sh enche")
    a.add_argument("--assets", type=Path, help="pasta com mel_filters.npz do openai-whisper (padrão: MODELO/assets)")
    a.add_argument("--correcoes", type=Path, action="append", default=[])
    a.add_argument("--vocabulario-frase", type=Path, default=None, help="uma frase com os termos do projeto (contexto)")
    a.add_argument("--sem-quadros", action="store_true")
    a.add_argument("--so-legendas", action="store_true", help="refazer as legendas da fala bruta guardada")
    args = a.parse_args()
    fixes = av.corrections(args.correcoes)
    if args.so_legendas:
        for n in args.videos:
            redo_subtitles(args.saida / n, fixes)
        return
    args.frase = args.vocabulario_frase.read_text(encoding="utf-8").strip() if args.vocabulario_frase else ""
    rec = W.Whisper(args.modelo / "sherpa-onnx-whisper-turbo", args.assets or args.modelo / "assets", threads=2)
    for v in args.videos:
        analyse(Path(v), args.saida, args, fixes, rec)


if __name__ == "__main__":
    main()
