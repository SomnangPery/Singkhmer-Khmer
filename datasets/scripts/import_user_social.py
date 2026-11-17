#!/usr/bin/env python3

import argparse
import glob
import json
import os
import re
import sys
from collections import Counter
from pathlib import Path

DATASETS_DIR = Path(__file__).resolve().parent.parent
PAIRS_DIR = DATASETS_DIR / "pairs"
DOWNLOADS_DIR = DATASETS_DIR / "downloads"
OUTPUT_PAIRS = PAIRS_DIR / "21_user_social_pairs.txt"

KHMER_CHAR_RE = re.compile(r"[\u1780-\u17FF]")
LATIN_TOKEN_RE = re.compile(r"^[a-zA-Z0-9_\'-]+$")


def fix_fb_encoding(text: str) -> str:
    try:
        return text.encode("latin1").decode("utf-8")
    except Exception:
        return text


def clean_text(text: str) -> str:
    text = re.sub(r"https?://\S+", "", text)
    text = re.sub(r"[@#]\S+", "", text)
    return text.strip()


def is_khmer_sentence(text: str) -> bool:
    khmer_count = len(KHMER_CHAR_RE.findall(text))
    return khmer_count >= 3 and (khmer_count / max(1, len(text))) > 0.3


def is_singkhmer_sentence(text: str) -> bool:
    latin_count = len(re.findall(r"[a-zA-Z]", text))
    khmer_count = len(KHMER_CHAR_RE.findall(text))
    return latin_count >= 4 and khmer_count == 0


def parse_facebook_json(filepath: Path):
    print(f"Reading Facebook export: {filepath}")
    messages = []
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            data = json.load(f)
        for msg in data.get("messages", []):
            content = msg.get("content")
            if content:
                fixed = fix_fb_encoding(content)
                cleaned = clean_text(fixed)
                if cleaned:
                    messages.append(cleaned)
    except Exception as e:
        print(f"Error parsing Facebook JSON {filepath}: {e}")
    return messages


def parse_telegram_json(filepath: Path):
    print(f"Reading Telegram export: {filepath}")
    messages = []
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            data = json.load(f)
        for msg in data.get("messages", []):
            text_field = msg.get("text", "")
            if isinstance(text_field, str):
                cleaned = clean_text(text_field)
                if cleaned:
                    messages.append(cleaned)
            elif isinstance(text_field, list):
                parts = []
                for entity in text_field:
                    if isinstance(entity, str):
                        parts.append(entity)
                    elif isinstance(entity, dict) and "text" in entity:
                        parts.append(entity["text"])
                cleaned = clean_text(" ".join(parts))
                if cleaned:
                    messages.append(cleaned)
    except Exception as e:
        print(f"Error parsing Telegram JSON {filepath}: {e}")
    return messages


def parse_text_or_json(filepath: Path):
    print(f"Reading text/TikTok export: {filepath}")
    lines = []
    try:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            first = f.read(1024).strip()
            f.seek(0)
            if first.startswith("{") or first.startswith("["):
                try:
                    data = json.load(f)
                    if isinstance(data, list):
                        for item in data:
                            if isinstance(item, str):
                                lines.append(clean_text(item))
                            elif isinstance(item, dict):
                                for k in ["text", "comment", "desc", "title", "caption"]:
                                    if k in item and isinstance(item[k], str):
                                        lines.append(clean_text(item[k]))
                    elif isinstance(data, dict):
                        for k in ["comments", "messages", "data", "items"]:
                            if k in data and isinstance(data[k], list):
                                for item in data[k]:
                                    if isinstance(item, str):
                                        lines.append(clean_text(item))
                                    elif isinstance(item, dict):
                                        for subk in ["text", "comment", "desc", "title", "caption"]:
                                            if subk in item and isinstance(item[subk], str):
                                                lines.append(clean_text(item[subk]))
                except Exception:
                    f.seek(0)
                    for line in f:
                        c = clean_text(line)
                        if c:
                            lines.append(c)
            else:
                for line in f:
                    c = clean_text(line)
                    if c:
                        lines.append(c)
    except Exception as e:
        print(f"Error parsing file {filepath}: {e}")
    return lines


def main():
    parser = argparse.ArgumentParser(description="Ingest Facebook, Telegram, TikTok data into Newkeyboard.")
    parser.add_argument("--fb", nargs="*", help="Facebook Messenger message_*.json file(s)")
    parser.add_argument("--tg", nargs="*", help="Telegram export result.json file(s)")
    parser.add_argument("--tiktok", nargs="*", help="TikTok export file(s) or comment text files")
    parser.add_argument("--auto", action="store_true", help="Auto-detect any exported files in datasets/downloads/")
    args = parser.parse_args()

    all_messages = []

    if args.fb:
        for path_str in args.fb:
            for p in glob.glob(path_str):
                all_messages.extend(parse_facebook_json(Path(p)))

    if args.tg:
        for path_str in args.tg:
            for p in glob.glob(path_str):
                all_messages.extend(parse_telegram_json(Path(p)))

    if args.tiktok:
        for path_str in args.tiktok:
            for p in glob.glob(path_str):
                all_messages.extend(parse_text_or_json(Path(p)))

    if args.auto or (not args.fb and not args.tg and not args.tiktok):
        for p in DOWNLOADS_DIR.glob("**/*.json"):
            name = p.name.lower()
            if "message" in name:
                all_messages.extend(parse_facebook_json(p))
            elif "result" in name:
                all_messages.extend(parse_telegram_json(p))
            elif "tiktok" in name:
                all_messages.extend(parse_text_or_json(p))

    print(f"\nCollected {len(all_messages)} total messages across inputs.")
    singkhmer_msgs = [m for m in all_messages if is_singkhmer_sentence(m)]
    khmer_msgs = [m for m in all_messages if is_khmer_sentence(m)]
    print(f"  - Singkhmer (Latin romanized) sentences: {len(singkhmer_msgs)}")
    print(f"  - Khmer script sentences: {len(khmer_msgs)}")

    token_counts = Counter()
    for m in singkhmer_msgs:
        tokens = m.lower().split()
        for t in tokens:
            t_clean = re.sub(r"[^a-z0-9]", "", t)
            if t_clean and len(t_clean) >= 2:
                token_counts[t_clean] += 1

    print(f"\nDiscovered {len(token_counts)} unique Singkhmer tokens.")
    if token_counts:
        print("Top 20 most frequent Singkhmer tokens:")
        for tok, count in token_counts.most_common(20):
            print(f"  {tok:<12} ({count}x)")


if __name__ == "__main__":
    main()
