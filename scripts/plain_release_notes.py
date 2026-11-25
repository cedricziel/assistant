"""Turns a GitHub release body into plain text for TestFlight and the App Store.

    gh release view "$TAG" --json body --jq .body | python3 scripts/plain_release_notes.py

TestFlight's "What to Test" and the App Store's "What's New" show the text as
it is, so release-please's markdown (headings, bold scopes, links, and the
trailing pull request and commit references) would reach users as raw symbols.
Links keep their text, the references are dropped, and the result is cut at a
line to fit the 4000 characters both accept.

App Review rejects "What's New" text that mentions other platforms (guideline
2.3.10), so lines about Android, Windows or Linux are left out, and a heading
with nothing left under it goes too.
"""

import re
import sys

LIMIT = 4000

# "([#93](url))" and "([2c79bd7](url))", which release-please appends to the end
# of a line. One in the middle of a sentence is part of the sentence.
_REFERENCE = re.compile(r"(?:\s*\(\[(?:#\d+|[0-9a-f]{7,40})\]\([^)]*\)\))+\s*$")
_LINK = re.compile(r"\[([^\]]*)\]\([^)]*\)")
_BOLD = re.compile(r"\*\*(.+?)\*\*|__(.+?)__")
_ITALIC = re.compile(r"\*(.+?)\*|(?<!\w)_(.+?)_(?!\w)")
_CODE = re.compile(r"`([^`]*)`")
_HEADING = re.compile(r"^#{1,6}\s+(.*)$")
_BULLET = re.compile(r"^\s*[*+-]\s+")
# "windows" alone is a Mac app's windows; the platform is "Windows" or the
# release-please scope at the start of an entry.
_OTHER_PLATFORM = re.compile(r"(?i:\b(?:android|linux)\b)|\bWindows\b|^windows:")


def _inline(text: str) -> str:
    text = _REFERENCE.sub("", text)
    text = _LINK.sub(r"\1", text)
    text = _BOLD.sub(lambda m: m.group(1) or m.group(2), text)
    text = _ITALIC.sub(lambda m: m.group(1) or m.group(2), text)
    return _CODE.sub(r"\1", text).strip()


class _Section:
    def __init__(self, title):
        self.title = title
        self.lines = []
        # A heading about another platform leaves out everything under it.
        self.other_platform = title is not None and bool(_OTHER_PLATFORM.search(title))
        self.left_out = False


def plain(markdown: str) -> str:
    # The first section has no heading: the text before any.
    sections = [_Section(None)]
    blank = False
    for raw in markdown.splitlines():
        if not raw.strip():
            blank = True
            continue
        heading = _HEADING.match(raw.strip())
        if heading:
            sections.append(_Section(_inline(heading.group(1))))
            blank = False
            continue
        section = sections[-1]
        bullet = _BULLET.match(raw)
        text = _inline(_BULLET.sub("", raw) if bullet else raw)
        if section.other_platform or _OTHER_PLATFORM.search(text):
            section.left_out = True
            continue
        if blank and section.lines:
            section.lines.append("")
        section.lines.append(f"- {text}" if bullet else text)
        blank = False

    lines: list[str] = []
    for section in sections:
        if section.other_platform or (section.left_out and not section.lines):
            continue
        if section.title is not None:
            if lines:
                lines.append("")
            lines.append(section.title)
        lines.extend(section.lines)

    text = "\n".join(lines)
    if len(text) <= LIMIT:
        return text

    kept: list[str] = []
    size = 0
    for line in lines:
        if size + len(line) + 1 > LIMIT - 2:
            break
        kept.append(line)
        size += len(line) + 1
    while kept and not kept[-1]:
        kept.pop()
    return "\n".join(kept) + "\n…"


def main() -> None:
    text = plain(sys.stdin.read())
    sys.stdout.write(text + ("\n" if text else ""))


if __name__ == "__main__":
    main()
