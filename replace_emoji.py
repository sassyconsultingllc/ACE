#!/usr/bin/env python3
"""Replace all inline Unicode emoji in Rust source with plain ASCII text.

This is Phase 1 of the emoji-to-SVG migration:
- Remove all inline emoji from UI strings
- Replace with clean ASCII text labels
- SVG icons will be rendered alongside text by the Icons system

Phase 2 (done in Rust code): wire up Icons::button/inline calls next to the text.
"""
import os
import re

SRC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src")

# Map of emoji/Unicode -> replacement text
# Format: (bytes_or_string_to_find, replacement)
REPLACEMENTS = [
    # ── Archive viewer ──────────────────────────
    # archive.rs UI buttons
    ('\u{1F4E6} Extract All', 'Extract All'),
    ('\u{1F4C4} Extract Selected', 'Extract Selected'),
    ('\u2795 New Archive', 'New Archive'),
    ('\u{1F4C1} Add Files', 'Add Files'),
    ('\u{1F332} Tree', 'Tree'),
    ('\u{1F50D}', 'Search'),  # standalone magnifying glass
    # archive.rs file-type icons (in fn file_icon)
    ('\u{1F4DD}', 'TXT'),   # memo
    ('\u{1F4BB}', 'CODE'),   # laptop
    ('\u{1F310}', 'WEB'),    # globe
    ('\u{1F4CB}', 'DATA'),   # clipboard
    ('\u{1F5BC}', 'IMG'),    # picture frame
    ('\u{1F3B5}', 'AUD'),    # musical note
    ('\u{1F3AC}', 'VID'),    # clapperboard
    ('\u{1F4D5}', 'PDF'),    # book
    ('\u{1F4C4}', 'DOC'),    # page
    ('\u{1F4CA}', 'XLS'),    # bar chart
    ('\u{1F4E6}', 'PKG'),    # package
    ('\u2699\uFE0F', 'EXE'),  # gear + variation selector
    ('\u2699', 'EXE'),       # gear without VS
    # archive.rs folder icons
    ('\u{1F4C2}', '[+]'),    # open folder
    ('\u{1F4C1}', '[-]'),    # closed folder

    # ── Audio viewer ────────────────────────────
    ('\u23EE', '|<'),    # skip to start
    ('\u23ED', '>|'),    # skip to end
    ('\u23F8', '||'),    # pause
    ('\u25B6', '>'),     # play / right triangle
    ('\u23F9', 'Stop'),  # stop
    ('\u{1F507}', 'Mute'),   # muted speaker
    ('\u{1F509}', 'Vol'),    # speaker low
    ('\u{1F50A}', 'Vol'),    # speaker high
    ('\u266B', '#'),     # beamed eighth notes

    # ── Video viewer ────────────────────────────
    ('\u2B1C', '[ ]'),   # white large square (exit fullscreen)
    ('\u26F6', '[#]'),   # square with corners (fullscreen)
    ('\u2139', 'i'),     # info source

    # ── PDF viewer ──────────────────────────────
    ('\u2796', '-'),     # heavy minus (zoom out)
    ('\u2795', '+'),     # heavy plus (zoom in)
    ('\u261B', '>'),     # black right pointing index (select)
    ('\u25BC', 'v'),     # down triangle
    ('\u25B2', '^'),     # up triangle
    ('\u25C0', '<'),     # left triangle
    ('\u26A0', '!'),     # warning sign

    # ── App.rs ──────────────────────────────────
    ('\u2715', 'X'),     # multiplication X (stop button)
    ('\u21BB', 'R'),     # clockwise arrow (reload)
    ('\u{1F4DA}', 'Sidebar'),  # books (sidebar toggle)
    ('\u{1F916}', 'AI'),       # robot (AI assistant)
    ('\u2193', 'v'),     # downwards arrow
    ('\u2191', '^'),     # upwards arrow
    ('\uFF0B', '+'),     # fullwidth plus
    ('\u2192', '->'),    # rightwards arrow
    ('\u2190', '<-'),    # leftwards arrow
    ('\u2714', '[ok]'),  # heavy check mark
    ('\u2705', '[ok]'),  # white heavy check mark
    ('\u274C', '[x]'),   # cross mark
    ('\u{1F7E2}', '(*)'),  # green circle
    ('\u26AA', '( )'),    # white circle
    ('\u2022', '*'),      # bullet point
    ('\u2013', '-'),      # en dash
    ('\u23F1\uFE0F', ''),  # stopwatch (in comment, just remove)
    ('\u23F1', ''),        # stopwatch without VS

    # ── Adblock ─────────────────────────────────
    ('\u{1F6E1}\uFE0F', ''),  # shield + VS
    ('\u{1F6E1}', ''),         # shield
    ('\u{1F6AB}', '[x]'),      # prohibited
    ('\u{1F4CA}', ''),         # bar chart
    ('\u{1F4CB}', ''),         # clipboard
    ('\u270F\uFE0F', ''),      # pencil + VS
    ('\u270F', ''),             # pencil

    # ── Settings / misc in app.rs ───────────────
    ('\u2699\uFE0F', ''),      # gear + VS
    ('\u{1F512}', ''),         # lock
    ('\u{1F464}', ''),         # user silhouette
    ('\u{1F504}', ''),         # refresh arrows
    ('\u{1F3E0}', ''),         # house

    # ── HTML renderer ───────────────────────────
    ('\u26A0\uFE0F', '!'),    # warning + VS

    # ── Spreadsheet ─────────────────────────────
    ('\u229E', '#'),     # squared plus (grid)
    ('\u2211', 'Sum'),   # summation

    # ── Text viewer ─────────────────────────────
    # summation already covered above

    # ── Document viewer ─────────────────────────
    # pilcrow (paragraph sign) - keep as text since it's a standard typographic symbol
    # degree sign - keep as text since it's standard

    # ── Sassy redesign extract ──────────────────
    ('\u2B05', '<'),     # leftwards black arrow
    ('\u27A1', '>'),     # rightwards black arrow
    ('\u{1F504}', 'R'),  # refresh
]

def replace_in_file(filepath, replacements):
    """Apply emoji replacements to a single file."""
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original = content
    count = 0
    for old, new in replacements:
        if old in content:
            n = content.count(old)
            content = content.replace(old, new)
            count += n

    if content != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        return count
    return 0

def main():
    total = 0
    changed_files = []

    for root, dirs, files in os.walk(SRC):
        # Skip target directory
        dirs[:] = [d for d in dirs if d != 'target']
        for f in files:
            if f.endswith('.rs'):
                path = os.path.join(root, f)
                count = replace_in_file(path, REPLACEMENTS)
                if count > 0:
                    changed_files.append((path, count))
                    total += count

    print(f"Replaced {total} emoji across {len(changed_files)} files:")
    for path, count in sorted(changed_files):
        relpath = os.path.relpath(path, SRC)
        print(f"  {relpath}: {count} replacements")

    # Also handle the wasm-demo and sassy-redesign dirs
    extra_dirs = [
        os.path.join(os.path.dirname(SRC), "wasm-demo", "src"),
        os.path.join(os.path.dirname(SRC), "sassy-redesign-extract"),
    ]
    for d in extra_dirs:
        if os.path.isdir(d):
            for root, dirs, files in os.walk(d):
                for f in files:
                    if f.endswith('.rs'):
                        path = os.path.join(root, f)
                        count = replace_in_file(path, REPLACEMENTS)
                        if count > 0:
                            relpath = os.path.relpath(path, os.path.dirname(SRC))
                            print(f"  {relpath}: {count} replacements")
                            total += count

    print(f"\nTotal: {total} emoji replaced")

if __name__ == "__main__":
    main()
