#!/usr/bin/env python3
"""Monta a pasta de saída de um projeto gravado do zero (uma pasta do Drive por projeto): legendas com o nome exato do vídeo,
transcrições, roteiros, dados, folhas e quadros, e a fala inteira no fim de cada relatório (no lugar de <!-- FALA -->).

Uso: python3 montar_saida_projeto.py ANALISES SAIDA "video 1.mp4" "video 2.mp4" ...
"""

import json
import shutil
import sys
import zipfile
from pathlib import Path


def main() -> None:
    analises, out, videos = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
    out.mkdir(parents=True, exist_ok=True)
    for video in videos:
        stem = video[:-4]  # com o espaço do fim, se houver: o .srt precisa do mesmo nome para o player abrir
        name = " ".join(stem.split())
        src = analises / name
        txt = (src / f"{name}.txt").read_text(encoding="utf-8")
        shutil.copy(src / f"{name}.srt", out / f"{stem}.srt")
        shutil.copy(src / f"{name}.txt", out / f"{name}.txt")
        dest = out / "analises" / name
        dest.mkdir(parents=True, exist_ok=True)
        shutil.copy(src / "roteiro.md", dest / "roteiro.md")
        data = json.loads((src / "dados.json").read_text(encoding="utf-8"))
        (dest / "dados.json").write_text(json.dumps(data, ensure_ascii=False, indent=1), encoding="utf-8")
        for f in sorted((src / "folhas").glob("*.jpg")):
            shutil.copy(f, dest / f.name)
        with zipfile.ZipFile(dest / "quadros.zip", "w", zipfile.ZIP_STORED) as z:
            for f in sorted((src / "quadros").glob("*.jpg")):
                z.write(f, f.name)
        rel = out / f"{name} - relatório.md"
        if rel.exists():
            body = rel.read_text(encoding="utf-8")
            fala = ("## A fala, na íntegra\n\nTranscrição revisada (Whisper large-v3-turbo), com o tempo de cada frase. É o mesmo "
                    "texto da legenda.\n\n```text\n" + txt.rstrip() + "\n```\n")
            if "<!-- FALA -->" in body:
                body = body.replace("<!-- FALA -->", fala)
            elif "## A fala, na íntegra" in body:
                body = body[:body.index("## A fala, na íntegra")] + fala
            rel.write_text(body, encoding="utf-8")
        print(f"{name}: ok", file=sys.stderr)


if __name__ == "__main__":
    main()
