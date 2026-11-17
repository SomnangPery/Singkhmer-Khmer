#!/usr/bin/env python3
"""Turns the words people shared from Bunny Keyboard into a pairs file.

People send words from the Share tab of the "My words" window (or with
"Wrong word?" in the popup). Each send is one answer to the Google Form, with
a "Words" question holding lines like

    srolanh = ស្រលាញ់
    tv = ធ្វើ (keyboard gave: ខ្ញុំចង់ទៅ; typed: nh jg tv)

and a permission question (the words are given to the public domain, CC0).

In the form's answer sheet: File -> Download -> Comma-separated values. Then:

    python datasets/scripts/import_shared_words.py answers.csv

It writes datasets/pairs/22_bunny_shared_words.txt (khmer=spelling,spelling),
keeping a pair only when at least --min-people answers sent it (default 2),
so one person cannot put a wrong word in everyone's dictionary. The pairs
then go into words-imported.tsv with the other pair files.
"""

import argparse
import csv
import re
import sys
from collections import defaultdict
from pathlib import Path

PAIRS_DIR = Path(__file__).resolve().parent.parent / "pairs"
OUTPUT = PAIRS_DIR / "22_bunny_shared_words.txt"

SPELLING = re.compile(r"^[a-z]+$")
KHMER = re.compile(r"^[ក-៿]+$")
LINE = re.compile(r"^\s*([A-Za-z]+)\s*[=:\t]\s*([ក-៿]+)")


def column(header, *words):
    """The first column whose title contains one of `words`."""
    for index, title in enumerate(header):
        if any(word in title.lower() for word in words):
            return index
    return None


def pairs_in(text):
    """The (spelling, khmer) pairs in one answer, each once."""
    found = set()
    for line in text.splitlines():
        match = LINE.match(line)
        if not match:
            continue
        spelling, khmer = match.group(1).lower(), match.group(2)
        if SPELLING.match(spelling) and KHMER.match(khmer):
            found.add((spelling, khmer))
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("answers", type=Path, help="the form's answers, downloaded as CSV")
    parser.add_argument("--min-people", type=int, default=2, help="answers that must agree on a pair (default 2)")
    parser.add_argument("--out", type=Path, default=OUTPUT)
    args = parser.parse_args()

    with open(args.answers, encoding="utf-8-sig", newline="") as file:
        rows = list(csv.reader(file))
    if not rows:
        sys.exit("The file is empty.")
    header, answers = rows[0], rows[1:]
    words_at = column(header, "word")
    permission_at = column(header, "permission", "cc0", "public domain")
    if words_at is None:
        sys.exit(f"No 'Words' column in {header}")

    people = defaultdict(int)
    without_permission = 0
    for answer in answers:
        if permission_at is not None and not answer[permission_at].strip():
            without_permission += 1
            continue
        for pair in pairs_in(answer[words_at] if words_at < len(answer) else ""):
            people[pair] += 1

    kept = defaultdict(list)
    for (spelling, khmer), count in sorted(people.items(), key=lambda item: (-item[1], item[0])):
        if count >= args.min_people:
            kept[khmer].append(spelling)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as out:
        out.write("# Source: words shared by Bunny Keyboard users through the Share tab\n")
        out.write("# License: CC0 1.0 (public domain), given by each sender on the form\n")
        out.write(f"# Kept: pairs sent by at least {args.min_people} answers\n\n")
        for khmer, spellings in kept.items():
            out.write(f"{khmer}={','.join(spellings)}\n")

    print(f"{len(answers)} answers, {without_permission} without permission (skipped)")
    print(f"{len(people)} different pairs, {sum(len(s) for s in kept.values())} kept "
          f"(sent by at least {args.min_people}) -> {args.out}")


if __name__ == "__main__":
    main()
