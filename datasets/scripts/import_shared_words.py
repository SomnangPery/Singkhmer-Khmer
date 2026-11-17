#!/usr/bin/env python3
"""Turns the words people shared from Bunny Keyboard into a pairs file.

People send words from the Share tab of the "My words" window (or with
"Wrong word?" in the popup). Each send is one answer to the Google Form. The
keyboard fills the form's Khmer question with lines like

    srolanh = ស្រលាញ់
    tv = ធ្វើ (keyboard gave: ខ្ញុំចង់ទៅ; typed: nh jg tv)

and its Singkhmer question with the spellings ("srolanh, tv"). People who
fill in the form by hand write a spelling in one and the Khmer word in the
other; those answers count too. If the form has a permission question (the
words are given to the public domain, CC0), answers without it are skipped.

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


def pairs_in(answer):
    """The (spelling, khmer) pairs in one answer (a list of its cells), each
    once: "spelling = Khmer" lines in any cell, or else the spellings in one
    cell and the Khmer words in another, in the same order."""
    found = set()
    for cell in answer:
        for line in cell.splitlines():
            match = LINE.match(line)
            if match and SPELLING.match(match.group(1).lower()):
                found.add((match.group(1).lower(), match.group(2)))
    if found:
        return found
    parts = [[part.strip() for part in re.split(r"[,\n]+", cell) if part.strip()] for cell in answer]
    latin = next((p for p in parts if p and all(SPELLING.match(word.lower()) for word in p)), [])
    khmer = next((p for p in parts if p and all(KHMER.match(word) for word in p)), [])
    if latin and len(latin) == len(khmer):
        found.update((spelling.lower(), word) for spelling, word in zip(latin, khmer))
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
    permission_at = column(header, "permission", "cc0", "public domain")
    timestamp_at = column(header, "timestamp")

    people = defaultdict(int)
    without_permission = 0
    for answer in answers:
        if permission_at is not None and not answer[permission_at].strip():
            without_permission += 1
            continue
        cells = [cell for at, cell in enumerate(answer) if at not in (permission_at, timestamp_at)]
        for pair in pairs_in(cells):
            people[pair] += 1

    kept = defaultdict(list)
    for (spelling, khmer), count in sorted(people.items(), key=lambda item: (-item[1], item[0])):
        if count >= args.min_people:
            kept[khmer].append(spelling)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as out:
        out.write("# Source: words shared by Bunny Keyboard users through the Share tab\n")
        if permission_at is not None:
            out.write("# License: CC0 1.0 (public domain), given by each sender on the form\n")
        else:
            out.write("# License: NOT CONFIRMED - the form had no permission question\n")
        out.write(f"# Kept: pairs sent by at least {args.min_people} answers\n\n")
        for khmer, spellings in kept.items():
            out.write(f"{khmer}={','.join(spellings)}\n")

    if permission_at is None:
        print("WARNING: the form has no permission (CC0) question, so it is not confirmed that "
              "senders gave their words to the public domain. Add one before publishing these words.")
    print(f"{len(answers)} answers, {without_permission} without permission (skipped)")
    print(f"{len(people)} different pairs, {sum(len(s) for s in kept.values())} kept "
          f"(sent by at least {args.min_people}) -> {args.out}")


if __name__ == "__main__":
    main()
