#!/usr/bin/env python3
"""Check the documents for the users that are in English and in Japanese (docs/dev/CONTRIBUTING.md, Languages of the documents).

A pair is NAME.md and NAME-ja.md in the same directory. Each must begin with the line that switches to the other
(`**English** | [日本語](NAME-ja.md)` and `[English](NAME.md) | **日本語**`), and the two must have the same number
of headings of each level (lines beginning with #, outside code blocks), so that a section added to one only is
found. A document that has the line but not its pair is an error too. It prints every problem and exits with 1 if
there is one; run it from the top of the repository (CI runs it, .github/workflows/ci.yml).
"""
import os
import re
import subprocess
import sys

HEADING = re.compile(r"(#{1,6}) ")


def headings(path):
    counts = {}
    fence = False
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("```"):
                fence = not fence
            elif not fence:
                m = HEADING.match(line)
                if m:
                    n = len(m.group(1))
                    counts[n] = counts.get(n, 0) + 1
    return counts


def first_line(path):
    with open(path, encoding="utf-8") as f:
        return f.readline().rstrip("\n")


def main():
    files = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "*.md"], capture_output=True, text=True, check=True).stdout.split()
    present = set(files)
    problems = []
    pairs = 0
    for ja in files:
        if not ja.endswith("-ja.md"):
            continue
        en = ja[:-len("-ja.md")] + ".md"
        name = os.path.basename(en)[:-3]
        en_line = f"**English** | [日本語]({name}-ja.md)"
        ja_line = f"[English]({name}.md) | **日本語**"
        if en not in present:
            if first_line(ja) == ja_line:
                problems.append(f"{ja}: it links to {en}, which is not there")
            continue
        pairs += 1
        if first_line(en) != en_line:
            problems.append(f"{en}: the first line is not {en_line}")
        if first_line(ja) != ja_line:
            problems.append(f"{ja}: the first line is not {ja_line}")
        h_en, h_ja = headings(en), headings(ja)
        for level in sorted(set(h_en) | set(h_ja)):
            if h_en.get(level, 0) != h_ja.get(level, 0):
                problems.append(f"{en} and {ja}: {h_en.get(level, 0)} and {h_ja.get(level, 0)} headings of level {level}")
    for en in files:
        if en.endswith(".md") and not en.endswith("-ja.md"):
            name = os.path.basename(en)[:-3]
            if first_line(en) == f"**English** | [日本語]({name}-ja.md)" and en[:-3] + "-ja.md" not in present:
                problems.append(f"{en}: it links to {en[:-3]}-ja.md, which is not there")
    for p in problems:
        print(f"doc-pairs.py: {p}")
    print(f"doc-pairs.py: {pairs} pairs of documents, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
