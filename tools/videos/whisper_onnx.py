"""Whisper large-v3-turbo (exportado em ONNX pelo sherpa-onnx) decodificado como o Whisper original.

O que o reconhecedor pronto do sherpa-onnx não faz e este faz: janelas de 30 s seguidas, com o texto já reconhecido e um
vocabulário como contexto (<|startofprev|>); tempos por frase (os tokens de tempo, com as regras do Whisper); volta com
temperatura quando o texto sai repetitivo ou improvável; e os silêncios longos cortados antes (Silero VAD), com os tempos
devolvidos ao vídeo original. Os dados do tokenizador e do espectrograma são os do pacote openai-whisper (PyPI).
"""

from __future__ import annotations

import base64
import zlib
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

SR = 16000
N_FFT = 400
HOP = 160
CHUNK_S = 30
N_FRAMES = CHUNK_S * SR // HOP  # 3000 quadros de 10 ms
TS_STEP = 0.02  # cada token de tempo vale 20 ms
INPUT_STRIDE = 2  # quadros do espectrograma por posição do codificador
PAT = r"""'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+"""
TEMPERATURES = (0.0, 0.2, 0.4, 0.6, 0.8, 1.0)


def softmax(x):
    x = x - np.max(x)
    e = np.exp(x)
    return e / e.sum()


def log_softmax(x):
    m = np.max(x)
    return x - m - np.log(np.sum(np.exp(x - m)))


def logsumexp(x):
    m = np.max(x)
    if not np.isfinite(m):
        return m
    return m + np.log(np.sum(np.exp(x - m)))


def compression_ratio(text: str) -> float:
    b = text.encode("utf-8")
    return len(b) / max(1, len(zlib.compress(b)))


@dataclass
class Decoded:
    tokens: list[int]
    text: str
    avg_logprob: float
    no_speech_prob: float
    compression_ratio: float
    temperature: float


@dataclass
class Segment:
    start: float
    end: float
    text: str
    avg_logprob: float = 0.0
    no_speech_prob: float = 0.0
    compression_ratio: float = 0.0
    temperature: float = 0.0
    tokens: list[int] = field(default_factory=list)


class Whisper:
    def __init__(self, model_dir: Path, assets_dir: Path, threads: int = 2, seed: int = 0):
        import onnxruntime as ort
        import tiktoken

        so = ort.SessionOptions()
        so.intra_op_num_threads = threads
        so.inter_op_num_threads = 1
        self.enc = ort.InferenceSession(str(model_dir / "turbo-encoder.int8.onnx"), so, providers=["CPUExecutionProvider"])
        self.dec = ort.InferenceSession(str(model_dir / "turbo-decoder.int8.onnx"), so, providers=["CPUExecutionProvider"])
        meta = self.enc.get_modelmeta().custom_metadata_map
        self.n_mels = int(meta["n_mels"])
        self.n_layer = int(meta["n_text_layer"])
        self.n_ctx = int(meta["n_text_ctx"])
        self.n_state = int(meta["n_text_state"])
        self.sot = int(meta["sot"])
        self.eot = int(meta["eot"])
        self.tok_transcribe = int(meta["transcribe"])
        self.tok_translate = int(meta["translate"])
        self.sot_prev = int(meta["sot_prev"])
        self.sot_lm = int(meta["sot_lm"])
        self.no_speech = int(meta["no_speech"])
        self.no_timestamps = int(meta["no_timestamps"])
        self.ts_begin = self.no_timestamps + 1
        self.n_vocab = int(meta["n_vocab"])
        codes = meta["all_language_codes"].split(",")
        langs = [int(x) for x in meta["all_language_tokens"].split(",")]
        self.lang = dict(zip(codes, langs))
        non_speech = [int(x) for x in meta["non_speech_tokens"].split(",")]
        self.suppress = np.array(sorted(set(non_speech + [self.tok_transcribe, self.tok_translate, self.sot, self.sot_prev,
                                                          self.sot_lm, self.no_speech])), dtype=np.int64)
        ranks = {}
        with open(model_dir / "turbo-tokens.txt", encoding="utf-8") as f:
            for line in f:
                parts = line.rstrip("\n").split(" ")
                ranks[base64.b64decode(parts[0])] = int(parts[-1])
        self.piece = {v: k for k, v in ranks.items()}
        self.bpe = tiktoken.Encoding("whisper-multilingual", pat_str=PAT, mergeable_ranks=ranks, special_tokens={})
        self.filters = np.load(assets_dir / "mel_filters.npz")[f"mel_{self.n_mels}"].astype(np.float32)
        self.rng = np.random.default_rng(seed)

    # ---------- texto ----------

    def text(self, tokens) -> str:
        return b"".join(self.piece[t] for t in tokens if t < self.eot).decode("utf-8", errors="replace")

    def encode(self, text: str) -> list[int]:
        return self.bpe.encode_ordinary(text)

    # ---------- áudio ----------

    def log_mel(self, audio: np.ndarray) -> np.ndarray:
        window = np.hanning(N_FFT + 1)[:-1].astype(np.float32)  # Hann periódica, como torch.hann_window
        padded = np.pad(audio.astype(np.float32), (N_FFT // 2, N_FFT // 2), mode="reflect")
        n = 1 + (len(padded) - N_FFT) // HOP
        frames = np.lib.stride_tricks.as_strided(padded, shape=(n, N_FFT),
                                                 strides=(padded.strides[0] * HOP, padded.strides[0]))
        power = np.abs(np.fft.rfft(frames * window, axis=1)) ** 2
        mel = self.filters @ power[:-1].T.astype(np.float32)
        log_spec = np.log10(np.clip(mel, 1e-10, None))
        log_spec = np.maximum(log_spec, log_spec.max() - 8.0)
        return ((log_spec + 4.0) / 4.0).astype(np.float32)

    # ---------- uma janela ----------

    def _rules(self, lg: np.ndarray, out: list[int], initial: bool) -> None:
        neg = -np.inf
        if initial:  # SuppressBlank
            lg[[220, self.eot]] = neg
        lg[self.suppress] = neg  # SuppressTokens
        lg[self.no_timestamps] = neg  # ApplyTimestampRules
        last_ts = len(out) >= 1 and out[-1] >= self.ts_begin
        penult_ts = len(out) < 2 or out[-2] >= self.ts_begin
        if last_ts:
            if penult_ts:
                lg[self.ts_begin:] = neg
            else:
                lg[:self.eot] = neg
        stamps = [t for t in out if t >= self.ts_begin]
        if stamps:
            last = stamps[-1] if (last_ts and not penult_ts) else stamps[-1] + 1
            lg[self.ts_begin:last] = neg
        if initial:
            lg[:self.ts_begin] = neg
            lg[self.ts_begin + 50 + 1:] = neg  # primeiro tempo até 1,0 s
        lp = log_softmax(lg)
        if logsumexp(lp[self.ts_begin:]) > np.max(lp[:self.ts_begin]):
            lg[:self.ts_begin] = neg

    def decode(self, mel_seg: np.ndarray, prompt: list[int], temperature: float, language: str = "pt") -> Decoded:
        cross_k, cross_v = self.enc.run(None, {"mel": mel_seg[None]})
        init: list[int] = []
        if prompt:
            init = [self.sot_prev] + prompt[-(self.n_ctx // 2 - 1):]
        sot_index = len(init)
        init += [self.sot, self.lang[language], self.tok_transcribe]
        sample_begin = len(init)
        k = np.zeros((self.n_layer, 1, self.n_ctx, self.n_state), np.float32)
        v = np.zeros_like(k)
        feed = np.array([init], np.int64)
        offset = 0
        out: list[int] = []
        sum_lp = 0.0
        no_speech_prob = 0.0
        for step in range(self.n_ctx // 2):
            logits, k, v = self.dec.run(None, {
                "tokens": feed, "in_n_layer_self_k_cache": k, "in_n_layer_self_v_cache": v,
                "n_layer_cross_k": cross_k, "n_layer_cross_v": cross_v, "offset": np.array([offset], np.int64)})
            if step == 0:
                no_speech_prob = float(softmax(logits[0, sot_index].astype(np.float64))[self.no_speech])
            offset += feed.shape[1]
            lg = logits[0, -1].astype(np.float64)
            self._rules(lg, out, step == 0)
            lp = log_softmax(lg)
            if temperature > 0:
                p = softmax(lg / temperature)
                tok = int(self.rng.choice(len(p), p=p))
            else:
                tok = int(np.argmax(lg))
            sum_lp += float(lp[tok])
            if tok == self.eot:
                break
            out.append(tok)
            if sample_begin + len(out) >= self.n_ctx:
                break
            feed = np.array([[tok]], np.int64)
        text = self.text(out)
        return Decoded(out, text, sum_lp / (len(out) + 1), no_speech_prob, compression_ratio(text), temperature)

    # ---------- o áudio inteiro ----------

    def transcribe(self, audio: np.ndarray, vocabulary: str = "", language: str = "pt") -> list[Segment]:
        """O áudio em frases com tempo, como o whisper.transcribe com carry_initial_prompt."""
        mel = self.log_mel(np.concatenate([audio, np.zeros(CHUNK_S * SR, np.float32)]))
        content = mel.shape[1] - N_FRAMES
        vocab = self.encode(" " + vocabulary.strip()) if vocabulary.strip() else []
        history: list[int] = []
        reset_since = 0
        seek = 0
        segments: list[Segment] = []
        while seek < content:
            offset_s = seek * HOP / SR
            size = min(N_FRAMES, content - seek)
            mel_seg = mel[:, seek:seek + N_FRAMES]
            if mel_seg.shape[1] < N_FRAMES:
                mel_seg = np.pad(mel_seg, ((0, 0), (0, N_FRAMES - mel_seg.shape[1])))
            room = self.n_ctx // 2 - 1 - len(vocab)
            prev = history[reset_since:]
            prompt = vocab + (prev[-room:] if room > 0 else [])
            r = None
            for t in TEMPERATURES:
                r = self.decode(np.ascontiguousarray(mel_seg), prompt, t, language)
                fallback = r.compression_ratio > 2.4 or r.avg_logprob < -1.0
                if r.no_speech_prob > 0.6:
                    fallback = False
                if not fallback:
                    break
            if r.no_speech_prob > 0.6 and r.avg_logprob < -1.0:
                seek += size
                continue
            toks = r.tokens
            is_ts = [t >= self.ts_begin for t in toks]
            single_end = len(toks) >= 2 and is_ts[-2:] == [False, True]
            consecutive = [i + 1 for i in range(len(toks) - 1) if is_ts[i] and is_ts[i + 1]]
            kept: list[int] = []
            new_seek = seek + size
            if consecutive:
                cuts = consecutive + ([len(toks)] if single_end else [])
                last = 0
                for cur in cuts:
                    part = toks[last:cur]
                    a = (part[0] - self.ts_begin) * TS_STEP if part[0] >= self.ts_begin else 0.0
                    b = (part[-1] - self.ts_begin) * TS_STEP if part[-1] >= self.ts_begin else a
                    segments.append(Segment(offset_s + a, offset_s + b, self.text(part).strip(), r.avg_logprob,
                                            r.no_speech_prob, r.compression_ratio, r.temperature, part))
                    kept.extend(part)
                    last = cur
                if not single_end:
                    new_seek = seek + (toks[last - 1] - self.ts_begin) * INPUT_STRIDE
            else:
                dur = size * HOP / SR
                stamps = [t for t in toks if t >= self.ts_begin]
                if stamps and stamps[-1] != self.ts_begin:
                    dur = (stamps[-1] - self.ts_begin) * TS_STEP
                segments.append(Segment(offset_s, offset_s + dur, self.text(toks).strip(), r.avg_logprob,
                                        r.no_speech_prob, r.compression_ratio, r.temperature, toks))
                kept = toks
            history.extend(kept)
            if r.temperature > 0.5:
                reset_since = len(history)
            seek = new_seek if new_seek > seek else seek + size
        return [s for s in segments if s.text and s.end > s.start]


# ---------- silêncios ----------

def speech_regions(audio: np.ndarray, vad_model: Path, min_silence: float = 2.0, pad: float = 0.4) -> list[list[float]]:
    """Os trechos com voz, com folga de `pad` s, juntando os separados por menos de `min_silence` s."""
    import sherpa_onnx

    cfg = sherpa_onnx.VadModelConfig()
    cfg.silero_vad.model = str(vad_model)
    cfg.silero_vad.threshold = 0.5
    cfg.silero_vad.min_silence_duration = 0.5
    cfg.silero_vad.min_speech_duration = 0.25
    cfg.silero_vad.max_speech_duration = 30
    cfg.silero_vad.window_size = 512
    cfg.sample_rate = SR
    cfg.num_threads = 1
    vad = sherpa_onnx.VoiceActivityDetector(cfg, buffer_size_in_seconds=900)
    found = []

    def drain():
        while not vad.empty():
            s = vad.front
            found.append((s.start / SR, (s.start + len(s.samples)) / SR))
            vad.pop()

    win = cfg.silero_vad.window_size
    for i in range(0, len(audio) - win + 1, win):
        vad.accept_waveform(audio[i:i + win])
        drain()
    vad.flush()
    drain()
    total = len(audio) / SR
    regions: list[list[float]] = []
    for a, b in found:
        a, b = max(0.0, a - pad), min(total, b + pad)
        if regions and a - regions[-1][1] < min_silence:
            regions[-1][1] = max(regions[-1][1], b)
        else:
            regions.append([a, b])
    return regions


def transcribe_speech(model: Whisper, audio: np.ndarray, vad_model: Path, vocabulary: str):
    """Transcreve só a fala (os silêncios longos cortados) e devolve os tempos do vídeo original."""
    regions = speech_regions(audio, vad_model)
    if not regions:
        return [], Timeline([], lambda t: t)
    pieces, table, pos = [], [], 0.0
    for a, b in regions:
        chunk = audio[int(a * SR):int(b * SR)]
        pieces.append(chunk)
        table.append((pos, a, len(chunk) / SR))
        pos += len(chunk) / SR
    joined = np.concatenate(pieces)

    def back(t: float) -> float:
        for cs, orig, ln in table:
            if t <= cs + ln + 1e-6:
                return orig + max(0.0, t - cs)
        cs, orig, _ = table[-1]
        return orig + (t - cs)

    segs = model.transcribe(joined, vocabulary)
    return segs, Timeline(table, back)


@dataclass
class Timeline:
    """A fala juntada (sem os silêncios longos) e o caminho de volta para os tempos do vídeo."""
    table: list
    back: object

    def boundaries(self) -> list[float]:
        return [cs for cs, _, _ in self.table[1:]]
