#!/usr/bin/env python3
"""Keep Jekyll from reading the Markdown of docs/ as Liquid (.github/workflows/pages.yml).

GitHub Pages runs Liquid on every page before Markdown, so a {{ or {% in the text, such as the syntax of
ConSh or the records of docs/dev/TESTS-RECORD-ja.md, is taken as a tag and stops the build. This replaces, on the
runner only, every {{ in the Markdown files under the directory given (docs) with {{ '{{' }} and every {%
with {{ '{%' }}: Liquid outputs each as the two characters, and Markdown then sees the text as it is in the
repository (in code too). The front matter, if any, is left as it is.

Each occurrence is replaced on its own, rather than the whole body being put between {% raw %} and
{% endraw %} (as before 2026-11-18): Liquid reads a {{ up to the next }, so a {{ with no } after it took the
} of the {% endraw %} at the end, and the raw block was never closed.
"""
import re
import sys
import os

OPEN = re.compile(r"\{([{%])")


def escape(text):
    head = ""
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end >= 0:
            head, text = text[:end + 5], text[end + 5:]
    return head + OPEN.sub(lambda m: "{{ '{" + m.group(1) + "' }}", text)


def main(top):
    n = 0
    for d, _, files in os.walk(top):
        for f in files:
            if not f.endswith(".md"):
                continue
            p = os.path.join(d, f)
            with open(p, encoding="utf-8") as fh:
                text = fh.read()
            new = escape(text)
            if new != text:
                with open(p, "w", encoding="utf-8") as fh:
                    fh.write(new)
                n += 1
    print(f"pages-escape.py: the braces of Liquid escaped in {n} Markdown files under {top}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "docs")
