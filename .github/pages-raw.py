#!/usr/bin/env python3
"""Keep Jekyll from reading the Markdown of docs/ as Liquid (.github/workflows/pages.yml).

GitHub Pages runs Liquid on every page before Markdown, so a {{ or {% in the text, such as the syntax of
ConSh in docs/consh/PLAN-ja.md, is taken as a tag and stops the build. This puts the body of each Markdown
file under the directory given (docs) between {% raw %} and {% endraw %}, on the runner only, so that the
files in the repository read as they are on GitHub. The front matter, if any, and a first heading stay
outside, so that jekyll-titles-from-headings still finds the title. A file that has its own
{% raw %} or {% endraw %} is left as it is (and stops the build if it has a {{ or {% outside them):
write about these tags in docs/ without the braces.
"""
import os
import sys

RAW, ENDRAW = "{% raw %}", "{% endraw %}"


def wrap(text):
    head = ""
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end >= 0:
            head, text = text[:end + 5], text[end + 5:]
    lines = text.split("\n")
    i = 0
    while i < len(lines) and not lines[i].strip():
        i += 1
    if i < len(lines) and lines[i].startswith("#") and "{{" not in lines[i] and "{%" not in lines[i]:
        i += 1
    before, body = "\n".join(lines[:i]), "\n".join(lines[i:])
    return head + (before + "\n" if before else "") + RAW + "\n" + body.rstrip("\n") + "\n" + ENDRAW + "\n"


def main(top):
    n = 0
    for d, _, files in os.walk(top):
        for f in files:
            if not f.endswith(".md"):
                continue
            p = os.path.join(d, f)
            with open(p, encoding="utf-8") as fh:
                text = fh.read()
            if RAW in text or ENDRAW in text:
                print(f"pages-raw.py: left as it is (has its own raw): {p}", file=sys.stderr)
                continue
            with open(p, "w", encoding="utf-8") as fh:
                fh.write(wrap(text))
            n += 1
    print(f"pages-raw.py: {n} Markdown files under {top} are read without Liquid")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "docs")
