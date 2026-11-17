#!/usr/bin/env python3
"""
Gathers more datasets:
1. Wikidata Cambodian entities (places, people, culture terms) -> 06_wikidata_cambodia.txt
2. Google Language Resources Khmer pronunciation lexicon (62k words) -> 07_google_km_pairs.txt
"""

import json
import re
import urllib.parse
import urllib.request
from pathlib import Path

DATASETS_DIR = Path(__file__).resolve().parent
DOWNLOADS_DIR = DATASETS_DIR / "downloads"


def is_khmer_word(word: str) -> bool:
    return bool(word) and all(
        ('\u1780' <= c <= '\u17d3') or c == '\u17dd' for c in word
    )


# ── 1. Wikidata Cambodian Entities ──────────────────────────────────
def fetch_wikidata():
    print("--- 1. Fetching Wikidata Cambodian Entities ---")
    query = """
    SELECT DISTINCT ?khmer ?en WHERE {
      { ?item wdt:P17 wd:Q424 } UNION { ?item wdt:P27 wd:Q424 } .
      ?item rdfs:label ?khmer FILTER(LANG(?khmer) = 'km') .
      ?item rdfs:label ?en FILTER(LANG(?en) = 'en') .
    }
    """
    url = "https://query.wikidata.org/sparql?format=json&query=" + urllib.parse.quote(query)
    req = urllib.request.Request(url, headers={"User-Agent": "NewkeyboardBot/1.0"})
    out_path = DATASETS_DIR / "06_wikidata_cambodia.txt"
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read())
        bindings = data.get("results", {}).get("bindings", [])
        pairs = {}
        for b in bindings:
            k = b.get("khmer", {}).get("value", "").strip()
            e = b.get("en", {}).get("value", "").strip()
            if is_khmer_word(k):
                clean_e = re.sub(r"[^a-zA-Z]", "", e).lower()
                if clean_e and len(clean_e) >= 2:
                    pairs.setdefault(k, set()).add(clean_e)

        with open(out_path, "w", encoding="utf-8") as f:
            f.write("# Source: https://query.wikidata.org/ (Cambodian entities: P17/P27 wd:Q424)\n")
            f.write("# License: CC0 (Public Domain)\n\n")
            total = 0
            for k, spellings in sorted(pairs.items()):
                f.write(f"{k}={','.join(sorted(spellings))}\n")
                total += len(spellings)
        print(f"  Extracted {total:,} spellings for {len(pairs):,} words -> {out_path.name}")
        return len(pairs)
    except Exception as e:
        print(f"  Wikidata error: {e}")
        return 0


# ── 2. Google Language Resources (62k words) ────────────────────────
def convert_google_km():
    print("\n--- 2. Converting Google Language Resources (62k words) ---")
    in_path = DOWNLOADS_DIR / "google_km_lexicon.tsv"
    out_path = DATASETS_DIR / "07_google_km_pairs.txt"
    if not in_path.exists():
        print(f"  Error: {in_path.name} not found")
        return 0

    phoneme_map = {
        "ɑ": "a", "ɑɑ": "aa", "ɓ": "b", "ɗ": "d", "ɔ": "o", "ɔɔ": "oo",
        "ə": "e", "əə": "eu", "ɛ": "e", "ɛɛ": "ae", "ɛə": "ea",
        "ɨ": "u", "ɨɨ": "ue", "ɨə": "uea", "ɲ": "nh", "ŋ": "ng", "ʔ": "",
        "j": "y", "c": "ch", "w": "v"
    }

    def phonemes_to_latin(p_str: str) -> str:
        tokens = p_str.split()
        res = []
        for t in tokens:
            if t == ".":
                continue
            res.append(phoneme_map.get(t, t))
        s = "".join(res).lower()
        return re.sub(r"[^a-z]", "", s)

    words = {}
    with open(in_path, "r", encoding="utf-8") as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            parts = line.strip().split("\t")
            if len(parts) >= 2:
                word = parts[0].strip()
                if is_khmer_word(word):
                    lat = phonemes_to_latin(parts[1])
                    if lat and len(lat) >= 1:
                        words.setdefault(word, set()).add(lat)

    with open(out_path, "w", encoding="utf-8") as f:
        f.write("# Source: https://github.com/google/language-resources (km/data/lexicon.tsv)\n")
        f.write("# License: CC BY 4.0 (Copyright 2018 Google Inc.)\n\n")
        total = 0
        for w, spellings in sorted(words.items()):
            f.write(f"{w}={','.join(sorted(spellings))}\n")
            total += len(spellings)
    print(f"  Extracted {total:,} spellings for {len(words):,} words -> {out_path.name}")
    return len(words)


def main():
    print("=" * 65)
    print("  Fetching More Datasets (Wikidata + Google 62k)")
    print("=" * 65)
    fetch_wikidata()
    convert_google_km()
    print("\nDone!")


if __name__ == "__main__":
    main()
