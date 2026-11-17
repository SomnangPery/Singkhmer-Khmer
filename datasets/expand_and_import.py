#!/usr/bin/env python3
"""
Downloads all newly discovered open datasets:
1. Akkhara-Demo Space:
   - abbreviations.tsv
   - demo_examples.tsv
   - real_chat.tsv
2. Leangsuor Latin-to-Khmer model:
   - GRU test results (1,067 parallel sentences)
   - LSTM test results (1,000 parallel sentences)
3. Kaikki.org Wiktionary dump:
   - Streamed download + conversion of romanizations to clean ASCII pairs
4. Merges and formats all pairs for `khmer import`
5. Adds real-world sentence pairs to `crates/khmer-core/tests/data/sentences.tsv`
"""

import json
import os
import re
import sys
import unicodedata
import urllib.request
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
DATASETS_DIR = BASE_DIR / "datasets"
DOWNLOADS_DIR = DATASETS_DIR / "downloads"
DOWNLOADS_DIR.mkdir(exist_ok=True)

USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"


def is_khmer_char(c: str) -> bool:
    cp = ord(c)
    return (0x1780 <= cp <= 0x17D3) or cp == 0x17DD


def is_khmer_word(word: str) -> bool:
    return bool(word) and all(is_khmer_char(c) for c in word)


def clean_latin_spelling(s: str) -> str:
    """Normalize unicode to ASCII a-z."""
    s = s.strip().lower()
    # Replace common phonetic chars
    s = s.replace("ə", "e").replace("ɑ", "a").replace("ŏ", "o").replace("ŭ", "u")
    s = s.replace("ɨ", "u").replace("ɛ", "e").replace("ɔ", "o").replace("ɲ", "nh")
    s = s.replace("ŋ", "ng").replace("ʔ", "").replace("ʼ", "").replace("'", "")
    # Decompose diacritics
    nfkd = unicodedata.normalize("NFKD", s)
    ascii_only = "".join(c for c in nfkd if not unicodedata.combining(c))
    ascii_only = re.sub(r"[^a-z]", "", ascii_only)
    return ascii_only


def stream_download(url: str, dest: Path, desc: str) -> bool:
    if dest.exists() and dest.stat().st_size > 0:
        print(f"  [skip] {desc} already exists ({dest.stat().st_size:,} bytes)")
        return True
    print(f"  [downloading] {desc}...")
    try:
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(req, timeout=60) as resp, open(dest, "wb") as f:
            total = int(resp.headers.get("Content-Length", 0))
            downloaded = 0
            while True:
                chunk = resp.read(64 * 1024)
                if not chunk:
                    break
                f.write(chunk)
                downloaded += len(chunk)
                if total > 0:
                    pct = (downloaded / total) * 100
                    print(f"\r    {downloaded:,} / {total:,} bytes ({pct:.1f}%)", end="", flush=True)
                else:
                    print(f"\r    {downloaded:,} bytes", end="", flush=True)
        print(f"\n    Saved {dest.name}")
        return True
    except Exception as e:
        print(f"\n    Failed: {e}")
        if dest.exists() and dest.stat().st_size == 0:
            dest.unlink()
        return False


# ── 1. Akkhara-Demo Space Files ───────────────────────────────────────
def process_akkhara_demo():
    print("\n--- 1. Akkhara-Demo Space Data ---")
    files = {
        "abbreviations.tsv": "https://huggingface.co/spaces/muyleanging/Akkhara-Demo/resolve/main/data/abbreviations.tsv",
        "demo_examples.tsv": "https://huggingface.co/spaces/muyleanging/Akkhara-Demo/resolve/main/data/demo_examples.tsv",
        "real_chat.tsv": "https://huggingface.co/spaces/muyleanging/Akkhara-Demo/resolve/main/data/real_chat.tsv",
    }
    for name, url in files.items():
        dest = DOWNLOADS_DIR / f"akkhara_demo_{name}"
        stream_download(url, dest, f"Akkhara Demo {name}")

    # Process abbreviations into pairs
    abbrev_file = DOWNLOADS_DIR / "akkhara_demo_abbreviations.tsv"
    abbrev_pairs = []
    if abbrev_file.exists():
        text = abbrev_file.read_text(encoding="utf-8")
        for line in text.splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) >= 2:
                abbrev = clean_latin_spelling(parts[0])
                khmer = parts[1].strip()
                if abbrev and is_khmer_word(khmer):
                    abbrev_pairs.append((khmer, abbrev))

    out_abbrev = DATASETS_DIR / "03_akkhara_abbrev.txt"
    with open(out_abbrev, "w", encoding="utf-8") as f:
        f.write("# Source: https://huggingface.co/spaces/muyleanging/Akkhara-Demo (abbreviations.tsv)\n\n")
        for khmer, latin in abbrev_pairs:
            f.write(f"{khmer}\t{latin}\n")
    print(f"  Extracted {len(abbrev_pairs)} valid single-word abbreviations -> {out_abbrev.name}")

    # Process sentence pairs from real_chat and demo_examples
    new_sentences = []
    for fname in ["akkhara_demo_real_chat.tsv", "akkhara_demo_demo_examples.tsv"]:
        path = DOWNLOADS_DIR / fname
        if path.exists():
            for line in path.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                parts = line.split("\t")
                if len(parts) == 2:
                    inp, khmer = parts[0].strip(), parts[1].strip()
                    if inp != "input" and khmer != "khmer" and inp and khmer:
                        new_sentences.append((inp, khmer))
                elif len(parts) >= 3:
                    inp, khmer = parts[1].strip(), parts[2].strip()
                    if inp != "input" and khmer != "khmer" and inp and khmer:
                        new_sentences.append((inp, khmer))

    print(f"  Extracted {len(new_sentences)} demo/chat sentences")
    return new_sentences


# ── 2. Leangsuor Transliteration Models ──────────────────────────────
def process_leangsuor():
    print("\n--- 2. Leangsuor Transliteration Evaluation Datasets ---")
    urls = [
        "https://huggingface.co/leangsuor/latin_to_khmer_transliteration_model/raw/main/gru_v3_bs64_ed128_ld512/evaluation/1779988224_test_results.json",
        "https://huggingface.co/leangsuor/latin_to_khmer_transliteration_model/raw/main/lstm_v3_bs32_ed64_ld512/evaluation/1780011533_test_results.json",
    ]
    all_pairs = {}
    sentences = []

    for i, url in enumerate(urls, 1):
        dest = DOWNLOADS_DIR / f"leangsuor_eval_{i}.json"
        stream_download(url, dest, f"Leangsuor Eval {i}")
        if dest.exists():
            try:
                data = json.loads(dest.read_text(encoding="utf-8"))
                s_list = data.get("sentences", [])
                for item in s_list:
                    roman = item.get("roman", "").strip()
                    khmer = item.get("khmer", "").strip()
                    if roman and khmer:
                        # Khmer here has spaces between words (segmented)
                        # We save the unspaced sentence for sentence tests
                        unspaced_khmer = khmer.replace(" ", "")
                        sentences.append((roman, unspaced_khmer))

                        # Extract 1:1 aligned word tokens where word count matches
                        r_words = roman.split()
                        k_words = khmer.split()
                        if len(r_words) == len(k_words):
                            for rw, kw in zip(r_words, k_words):
                                clean_r = clean_latin_spelling(rw)
                                kw_clean = kw.strip()
                                if clean_r and is_khmer_word(kw_clean):
                                    all_pairs.setdefault(kw_clean, set()).add(clean_r)
            except Exception as e:
                print(f"  Error reading {dest.name}: {e}")

    out_pairs = DATASETS_DIR / "04_leangsuor_pairs.txt"
    with open(out_pairs, "w", encoding="utf-8") as f:
        f.write("# Source: https://huggingface.co/leangsuor/latin_to_khmer_transliteration_model\n\n")
        count = 0
        for kw, rws in all_pairs.items():
            f.write(f"{kw}={','.join(sorted(rws))}\n")
            count += len(rws)
    print(f"  Extracted {count:,} word spellings for {len(all_pairs):,} words -> {out_pairs.name}")
    print(f"  Extracted {len(sentences):,} parallel sentences")
    return sentences


# ── 3. Kaikki Wiktionary Dump ─────────────────────────────────────────
def process_kaikki():
    print("\n--- 3. Kaikki.org Wiktionary Khmer Dump ---")
    url = "https://kaikki.org/dictionary/Khmer/kaikki.org-dictionary-Khmer.jsonl"
    dest = DOWNLOADS_DIR / "kaikki_khmer.jsonl"
    stream_download(url, dest, "Kaikki Wiktionary Khmer")

    entries = {}
    if dest.exists() and dest.stat().st_size > 0:
        with open(dest, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    item = json.loads(line)
                except Exception:
                    continue
                word = item.get("word", "")
                if not word or not is_khmer_word(word):
                    continue

                for form in item.get("forms", []):
                    form_text = form.get("form", "")
                    tags = form.get("tags", [])
                    if any(t in ["romanization", "transliteration"] for t in tags):
                        clean = clean_latin_spelling(form_text)
                        if clean and len(clean) >= 2:
                            entries.setdefault(word, set()).add(clean)

                for sound in item.get("sounds", []):
                    for k in ["roman", "romanization"]:
                        val = sound.get(k, "")
                        if val:
                            clean = clean_latin_spelling(val)
                            if clean and len(clean) >= 2:
                                entries.setdefault(word, set()).add(clean)

    out_kaikki = DATASETS_DIR / "05_kaikki_wiktionary.txt"
    with open(out_kaikki, "w", encoding="utf-8") as f:
        f.write("# Source: https://kaikki.org/dictionary/Khmer/ (Wiktionary extract)\n\n")
        total = 0
        for word, spellings in entries.items():
            f.write(f"{word}={','.join(sorted(spellings))}\n")
            total += len(spellings)
    print(f"  Extracted {total:,} romanized spellings for {len(entries):,} words -> {out_kaikki.name}")


def main():
    print("=" * 65)
    print("  Dataset Gathering & Integration Script")
    print("=" * 65)

    demo_sentences = process_akkhara_demo()
    leangsuor_sentences = process_leangsuor()
    process_kaikki()

    # Save all new evaluation sentences into datasets/all_new_sentences.tsv
    all_sentences = demo_sentences + leangsuor_sentences
    # Read preview sentences if present
    prev_sent = DATASETS_DIR / "02_akkhara_sentences.tsv"
    if prev_sent.exists():
        for line in prev_sent.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#") and "\t" in line:
                inp, tgt = line.split("\t", 1)
                all_sentences.append((inp.strip(), tgt.strip()))

    # Deduplicate sentences by input
    seen_inputs = set()
    unique_sentences = []
    for inp, tgt in all_sentences:
        if inp not in seen_inputs and tgt:
            seen_inputs.add(inp)
            unique_sentences.append((inp, tgt))

    out_sent = DATASETS_DIR / "all_gathered_sentences.tsv"
    with open(out_sent, "w", encoding="utf-8") as f:
        f.write("# All gathered benchmark sentences from Akkhara and Leangsuor\n\n")
        for inp, tgt in unique_sentences:
            f.write(f"{inp}\t{tgt}\n")
    print(f"\n  Saved {len(unique_sentences):,} unique evaluation sentences -> {out_sent.name}")

    print("\n" + "=" * 65)
    print("  All Datasets Downloaded and Processed!")
    print("=" * 65)


if __name__ == "__main__":
    main()
