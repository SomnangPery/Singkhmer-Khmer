#!/usr/bin/env python3
"""
Download and convert all available Singkhmer-to-Khmer datasets into
the pair-file format that `khmer import` expects.

Run with:
    python datasets/fetch_all.py

Requires network access (run outside sandbox).
"""

import json
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path

DATASETS_DIR = Path(__file__).parent
DOWNLOADS_DIR = DATASETS_DIR / "downloads"
DOWNLOADS_DIR.mkdir(exist_ok=True)


def download(url: str, dest: Path, description: str) -> bool:
    """Download a URL to a file. Returns True on success."""
    if dest.exists() and dest.stat().st_size > 0:
        print(f"  [skip] {description} already downloaded: {dest.name}")
        return True
    print(f"  [download] {description}...")
    print(f"    URL: {url}")
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = resp.read()
        dest.write_bytes(data)
        print(f"    Saved: {dest} ({len(data):,} bytes)")
        return True
    except (urllib.error.URLError, urllib.error.HTTPError, OSError) as e:
        print(f"    FAILED: {e}")
        # Remove empty files
        if dest.exists() and dest.stat().st_size == 0:
            dest.unlink()
        return False


def is_khmer_char(c: str) -> bool:
    cp = ord(c)
    return (0x1780 <= cp <= 0x17D3) or cp == 0x17DD


def is_khmer_word(word: str) -> bool:
    return bool(word) and all(is_khmer_char(c) for c in word)


def is_latin_spelling(s: str) -> bool:
    return bool(s) and all(c.isascii() and c.isalpha() for c in s)


def write_pairs(pairs, out_path, header=""):
    with open(out_path, "w", encoding="utf-8") as f:
        if header:
            for line in header.strip().split("\n"):
                f.write(f"# {line}\n")
            f.write("\n")
        written = 0
        for khmer, latin in pairs:
            f.write(f"{khmer}\t{latin}\n")
            written += 1
    print(f"  [wrote] {out_path.name}: {written:,} pairs")


def write_multi_spelling(entries, out_path, header=""):
    with open(out_path, "w", encoding="utf-8") as f:
        if header:
            for line in header.strip().split("\n"):
                f.write(f"# {line}\n")
            f.write("\n")
        for khmer, spellings in entries.items():
            f.write(f"{khmer}={','.join(spellings)}\n")
    total = sum(len(sp) for sp in entries.values())
    print(f"  [wrote] {out_path.name}: {total:,} spellings for {len(entries):,} words")


def parse_generic_pairs(text: str) -> list:
    """Parse a text file with Khmer/Latin pairs in various formats."""
    pairs = []
    for line in text.strip().split("\n"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        for sep in ["\t", ",", ":"]:
            if sep in line:
                parts = line.split(sep)
                if len(parts) >= 2:
                    a, b = parts[0].strip(), parts[1].strip()
                    if is_khmer_word(a) and is_latin_spelling(b):
                        pairs.append((a, b.lower()))
                    elif is_khmer_word(b) and is_latin_spelling(a):
                        pairs.append((b, a.lower()))
                break
    return pairs


# ── 1. Chhunneng/khmer-text-transliteration ──────────────────────────
def fetch_chhunneng():
    print("\n═══ 1. Chhunneng/khmer-text-transliteration ═══")
    base = "https://raw.githubusercontent.com/Chhunneng/khmer-text-transliteration/main/"
    files = [
        "khmer_text_transliteration/data/raw/eng_khm_data.txt",
    ]
    all_pairs = []
    for fname in files:
        dest = DOWNLOADS_DIR / fname.replace("/", "_")
        if download(base + fname, dest, fname):
            text = dest.read_text(encoding="utf-8", errors="replace")
            all_pairs.extend(parse_generic_pairs(text))
    if all_pairs:
        write_pairs(all_pairs, DATASETS_DIR / "01_chhunneng.txt",
                    "Source: https://github.com/Chhunneng/khmer-text-transliteration\n"
                    "License: MIT, Copyright (c) 2025 Chrea Chanchhunneng")
    return len(all_pairs)


# ── 2. Akkhara-Sing-Khmer Preview (HuggingFace rows API) ────────────
def fetch_akkhara_preview():
    print("\n═══ 2. Akkhara-Sing-Khmer Preview ═══")
    # Use the datasets-server rows API which returns JSON directly
    base_url = "https://datasets-server.huggingface.co/rows"
    configs = {
        "pairs": "default",
        "lexicon": "lexicon",
    }

    all_sentence_pairs = []
    all_lexicon_entries = {}

    for name, config in configs.items():
        # Try to get up to 1000 rows
        for offset in range(0, 2000, 100):
            url = f"{base_url}?dataset=muyleanging/Akkhara-Sing-Khmer-Preview&config={config}&split=train&offset={offset}&length=100"
            dest = DOWNLOADS_DIR / f"akkhara_preview_{name}_{offset}.json"
            if not download(url, dest, f"akkhara preview {name} offset={offset}"):
                break

            try:
                data = json.loads(dest.read_text(encoding="utf-8"))
                if "error" in data:
                    print(f"    API error: {data['error']}")
                    break
                rows = data.get("rows", [])
                if not rows:
                    break

                for r in rows:
                    row = r.get("row", r)
                    if name == "pairs":
                        # Try various field names
                        inp = ""
                        target = ""
                        for k in ["input", "sing_khmer", "romanized", "latin", "source"]:
                            if k in row and isinstance(row[k], str):
                                val = row[k].strip()
                                if val and not is_khmer_word(val.replace(" ", "")):
                                    inp = val
                                    break
                        for k in ["target", "khmer", "khmer_script", "output"]:
                            if k in row and isinstance(row[k], str):
                                val = row[k].strip()
                                if val:
                                    target = val
                                    break
                        if inp and target:
                            all_sentence_pairs.append((inp, target))
                    elif name == "lexicon":
                        khmer = ""
                        spellings_raw = ""
                        for k in ["khmer", "word", "khmer_word"]:
                            if k in row and isinstance(row[k], str):
                                khmer = row[k].strip()
                                break
                        for k in ["chat_spelling", "romanized", "spelling", "spellings", "latin"]:
                            if k in row:
                                spellings_raw = row[k]
                                break
                        if khmer and spellings_raw:
                            if isinstance(spellings_raw, list):
                                sp_list = spellings_raw
                            else:
                                sp_list = [s.strip() for s in str(spellings_raw).split(",")]
                            sp_list = [s.lower() for s in sp_list if is_latin_spelling(s)]
                            if is_khmer_word(khmer) and sp_list:
                                if khmer in all_lexicon_entries:
                                    existing = set(all_lexicon_entries[khmer])
                                    existing.update(sp_list)
                                    all_lexicon_entries[khmer] = list(existing)
                                else:
                                    all_lexicon_entries[khmer] = sp_list

                # If fewer than 100 rows returned, we've reached the end
                if len(rows) < 100:
                    break
            except (json.JSONDecodeError, KeyError, TypeError) as e:
                print(f"    Parse error at offset {offset}: {e}")
                break

    # Print schema of first result for debugging
    first_file = DOWNLOADS_DIR / "akkhara_preview_pairs_0.json"
    if first_file.exists():
        try:
            data = json.loads(first_file.read_text(encoding="utf-8"))
            rows = data.get("rows", [])
            if rows:
                row = rows[0].get("row", rows[0])
                print(f"  [schema] pairs keys: {list(row.keys())}")
                # Print first row for debugging
                print(f"  [sample] first row: {json.dumps(row, ensure_ascii=False)[:200]}")
        except Exception:
            pass

    first_lex = DOWNLOADS_DIR / "akkhara_preview_lexicon_0.json"
    if first_lex.exists():
        try:
            data = json.loads(first_lex.read_text(encoding="utf-8"))
            rows = data.get("rows", [])
            if rows:
                row = rows[0].get("row", rows[0])
                print(f"  [schema] lexicon keys: {list(row.keys())}")
                print(f"  [sample] first row: {json.dumps(row, ensure_ascii=False)[:200]}")
        except Exception:
            pass

    # Write sentence pairs for test data
    if all_sentence_pairs:
        with open(DATASETS_DIR / "02_akkhara_sentences.tsv", "w", encoding="utf-8") as f:
            f.write("# Source: https://huggingface.co/datasets/muyleanging/Akkhara-Sing-Khmer-Preview\n")
            f.write("# License: CC BY-SA 4.0\n\n")
            for romanized, khmer in all_sentence_pairs:
                f.write(f"{romanized}\t{khmer}\n")
        print(f"  [wrote] 02_akkhara_sentences.tsv: {len(all_sentence_pairs)} sentences")

    # Write lexicon entries as import-ready pairs
    if all_lexicon_entries:
        write_multi_spelling(all_lexicon_entries, DATASETS_DIR / "02_akkhara_lexicon.txt",
                           "Source: https://huggingface.co/datasets/muyleanging/Akkhara-Sing-Khmer-Preview\n"
                           "License: CC BY-SA 4.0")

    return len(all_sentence_pairs) + sum(len(v) for v in all_lexicon_entries.values())


# ── 3. Akkhara-Sing-Khmer FULL (may need access request) ────────────
def fetch_akkhara_full():
    print("\n═══ 3. Akkhara-Sing-Khmer (full dataset, may need access) ═══")
    base_url = "https://datasets-server.huggingface.co/rows"
    configs = ["default", "lexicon"]
    total = 0

    for config in configs:
        url = f"{base_url}?dataset=muyleanging/Akkhara-Sing-Khmer&config={config}&split=train&offset=0&length=100"
        dest = DOWNLOADS_DIR / f"akkhara_full_{config}_0.json"
        if download(url, dest, f"akkhara full {config}"):
            try:
                data = json.loads(dest.read_text(encoding="utf-8"))
                if "error" in data:
                    print(f"    Restricted: {data['error'][:120]}")
                    print(f"    → Request access at: https://huggingface.co/datasets/muyleanging/Akkhara-Sing-Khmer")
                else:
                    num_rows = data.get("num_rows_total", len(data.get("rows", [])))
                    print(f"    Config '{config}': {num_rows:,} total rows available")
                    total += num_rows
                    rows = data.get("rows", [])
                    if rows:
                        row = rows[0].get("row", rows[0])
                        print(f"    Schema: {list(row.keys())}")
                        print(f"    Sample: {json.dumps(row, ensure_ascii=False)[:200]}")
            except Exception as e:
                print(f"    Parse error: {e}")
    return total


# ── 4. Seanghay khmer-latin-name-transformer ─────────────────────────
def fetch_seanghay_names():
    print("\n═══ 4. seanghay/khmer-latin-name-transformer ═══")
    # Try to find data files in the repo
    urls = [
        ("https://raw.githubusercontent.com/seanghay/khmer-latin-name-transformer/main/data/names.tsv", "names.tsv"),
        ("https://raw.githubusercontent.com/seanghay/khmer-latin-name-transformer/main/data/train.tsv", "train.tsv"),
        ("https://raw.githubusercontent.com/seanghay/khmer-latin-name-transformer/main/names.tsv", "names_root.tsv"),
        ("https://raw.githubusercontent.com/seanghay/khmer-latin-name-transformer/main/data/pairs.txt", "pairs.txt"),
        ("https://raw.githubusercontent.com/seanghay/khmer-latin-name-transformer/main/data/names.txt", "names.txt"),
        ("https://raw.githubusercontent.com/seanghay/khmer-latin-name-transformer/main/data/test.tsv", "test.tsv"),
        ("https://raw.githubusercontent.com/seanghay/khmer-latin-name-transformer/main/data/raw.tsv", "raw.tsv"),
    ]
    all_pairs = []
    for url, name in urls:
        dest = DOWNLOADS_DIR / f"seanghay_{name}"
        if download(url, dest, f"seanghay {name}"):
            text = dest.read_text(encoding="utf-8", errors="replace")
            all_pairs.extend(parse_generic_pairs(text))

    if all_pairs:
        write_pairs(all_pairs, DATASETS_DIR / "04_seanghay_names.txt",
                    "Source: https://github.com/seanghay/khmer-latin-name-transformer\n"
                    "License: Check repository")
    return len(all_pairs)


# ── 5. SIL/SEALang frequency list ───────────────────────────────────
def fetch_sil_frequency():
    print("\n═══ 5. SIL NRSI / SEALang frequency list ═══")
    urls = [
        "https://raw.githubusercontent.com/silnrsi/khmerlbdict/master/src/seafreq.txt",
        "https://raw.githubusercontent.com/silnrsi/khmerlbdict/main/src/seafreq.txt",
    ]
    for url in urls:
        dest = DOWNLOADS_DIR / "sil_seafreq.txt"
        if download(url, dest, "SIL SEALang frequency list"):
            text = dest.read_text(encoding="utf-8", errors="replace")
            count = sum(1 for line in text.strip().split("\n")
                       if line.strip() and not line.startswith("#"))
            print(f"  [info] {count} frequency entries")
            print(f"         Use with: cargo run -p khmer-cli -- import ... --freq datasets/downloads/sil_seafreq.txt")
            return count
    return 0


# ── 6. Google language-resources Khmer ───────────────────────────────
def fetch_google_km():
    print("\n═══ 6. google/language-resources (km) ═══")
    urls = [
        "https://raw.githubusercontent.com/google/language-resources/master/km/data/lexicon.tsv",
        "https://raw.githubusercontent.com/google/language-resources/master/km/lexicon.tsv",
    ]
    for url in urls:
        dest = DOWNLOADS_DIR / "google_km_lexicon.tsv"
        if download(url, dest, "Google KM lexicon"):
            text = dest.read_text(encoding="utf-8", errors="replace")
            entries = 0
            for line in text.strip().split("\n"):
                if not line.startswith("#") and line.strip():
                    entries += 1
            print(f"  [info] {entries} phonemic entries (reference, not chat spellings)")
            return entries
    return 0


# ── 7. Kaikki.org Wiktionary Khmer ──────────────────────────────────
def fetch_kaikki():
    print("\n═══ 7. Kaikki.org Wiktionary Khmer dump ═══")
    url = "https://kaikki.org/dictionary/Khmer/kaikki.org-dictionary-Khmer.jsonl"
    dest = DOWNLOADS_DIR / "kaikki_khmer.jsonl"
    if not download(url, dest, "Kaikki Wiktionary Khmer JSONL"):
        return 0

    text = dest.read_text(encoding="utf-8", errors="replace")
    entries = {}
    total_pairs = 0

    for line_num, line in enumerate(text.strip().split("\n"), 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue

        word = entry.get("word", "")
        if not word or not is_khmer_word(word):
            continue

        romanizations = set()

        # "forms" with romanization tags
        for form in entry.get("forms", []):
            form_text = form.get("form", "")
            tags = form.get("tags", [])
            if any(t in ["romanization", "transliteration", "Latn"] for t in tags):
                clean = form_text.lower().replace("-", "").replace(" ", "").replace("'", "")
                if clean and is_latin_spelling(clean):
                    romanizations.add(clean)

        # "sounds" → "roman" field
        for sound in entry.get("sounds", []):
            for key in ["roman", "romanization"]:
                val = sound.get(key, "")
                if val:
                    clean = val.lower().replace("-", "").replace(" ", "").replace("'", "")
                    if clean and is_latin_spelling(clean):
                        romanizations.add(clean)

        if romanizations:
            entries[word] = list(romanizations)
            total_pairs += len(romanizations)

    if entries:
        write_multi_spelling(entries, DATASETS_DIR / "07_kaikki_wiktionary.txt",
                           "Source: https://kaikki.org/dictionary/Khmer/ (Wiktionary extract)\n"
                           "License: CC BY-SA 3.0 / GFDL")
    else:
        print(f"  [info] No romanization pairs extracted (Wiktionary Khmer entries may lack Latin transliterations)")
        print(f"         The download contains {line_num:,} Khmer word entries for other reference use")

    return total_pairs


# ── 8. phonsobon/khmer-pronunciation-dictionary (HuggingFace) ───────
def fetch_phonsobon():
    print("\n═══ 8. phonsobon/khmer-pronunciation-dictionary ═══")
    url = "https://datasets-server.huggingface.co/rows?dataset=phonsobon/khmer-pronunciation-dictionary&config=default&split=train&offset=0&length=100"
    dest = DOWNLOADS_DIR / "phonsobon_sample.json"
    if download(url, dest, "phonsobon pronunciation dictionary"):
        try:
            data = json.loads(dest.read_text(encoding="utf-8"))
            if "error" in data:
                print(f"    Error: {data['error'][:100]}")
                return 0
            rows = data.get("rows", [])
            num_total = data.get("num_rows_total", len(rows))
            if rows:
                row = rows[0].get("row", rows[0])
                print(f"  [schema] {list(row.keys())}")
                print(f"  [sample] {json.dumps(row, ensure_ascii=False)[:200]}")
                print(f"  [info] {num_total:,} total rows available via HuggingFace API")
            return num_total
        except Exception as e:
            print(f"  Error: {e}")
    return 0


# ── 9. seanghay/khmer-dictionary-44k ────────────────────────────────
def fetch_seanghay_dict():
    print("\n═══ 9. seanghay/khmer-dictionary-44k ═══")
    urls = [
        "https://raw.githubusercontent.com/seanghay/khmer-dictionary-44k/main/dictionary.json",
        "https://raw.githubusercontent.com/seanghay/khmer-dictionary-44k/main/data.json",
        "https://raw.githubusercontent.com/seanghay/khmer-dictionary-44k/main/words.txt",
    ]
    for url in urls:
        name = url.split("/")[-1]
        dest = DOWNLOADS_DIR / f"seanghay_dict_{name}"
        if download(url, dest, f"seanghay dictionary {name}"):
            try:
                if name.endswith(".json"):
                    data = json.loads(dest.read_text(encoding="utf-8"))
                    if isinstance(data, list):
                        count = len(data)
                    elif isinstance(data, dict):
                        count = len(data)
                    else:
                        count = 0
                    print(f"  [info] {count:,} entries (Khmer-only dictionary, for word list reference)")
                else:
                    text = dest.read_text(encoding="utf-8", errors="replace")
                    count = sum(1 for l in text.strip().split("\n") if l.strip())
                    print(f"  [info] {count:,} words (for word list reference)")
                return count
            except Exception as e:
                print(f"  Error: {e}")
    return 0


# ── Main ─────────────────────────────────────────────────────────────
def main():
    print("=" * 70)
    print("  Singkhmer → Khmer Dataset Downloader")
    print("  For: Newkeyboard / khmer-core")
    print("=" * 70)

    results = {}
    results["1. Chhunneng transliteration"]   = fetch_chhunneng()
    results["2. Akkhara-Sing-Khmer Preview"]  = fetch_akkhara_preview()
    results["3. Akkhara-Sing-Khmer Full"]     = fetch_akkhara_full()
    results["4. Seanghay name transformer"]   = fetch_seanghay_names()
    results["5. SIL/SEALang frequency"]       = fetch_sil_frequency()
    results["6. Google language-resources"]    = fetch_google_km()
    results["7. Kaikki Wiktionary"]           = fetch_kaikki()
    results["8. phonsobon pronunciation"]     = fetch_phonsobon()
    results["9. seanghay dictionary-44k"]     = fetch_seanghay_dict()

    print("\n" + "=" * 70)
    print("  Summary")
    print("=" * 70)
    for name, count in results.items():
        status = f"{count:>8,} entries" if count > 0 else "  no data / needs access"
        print(f"  {name:40s} {status}")

    print(f"\n  Output files in: {DATASETS_DIR}")

    # List output files
    out_files = sorted(f for f in DATASETS_DIR.iterdir()
                       if f.suffix in (".txt", ".tsv") and f.name[0].isdigit())
    if out_files:
        print("\n  Import-ready files:")
        for f in out_files:
            size = f.stat().st_size
            print(f"    {f.name:40s} ({size:>10,} bytes)")

    print("\n  Next steps:")
    print("    1. Review the downloaded files in datasets/")
    print("    2. Import with:")
    print("       cargo run -p khmer-cli -- import datasets/01_chhunneng.txt \\")
    print("         datasets/02_akkhara_lexicon.txt \\")
    print("         crates/khmer-core/data/words-imported.tsv \\")
    print("         --freq datasets/downloads/sil_seafreq.txt \\")
    print('         --note "Source: Chhunneng (MIT), Akkhara (CC BY-SA 4.0)"')
    print("    3. Copy useful sentence pairs to: crates/khmer-core/tests/data/sentences.tsv")
    print("    4. Run: cargo test")
    print()


if __name__ == "__main__":
    main()
