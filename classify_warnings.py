#!/usr/bin/env python
"""Classify cargo check warnings by type."""
import re, sys

with open(r"V:\sassy-browser-FIXED\warn_lines.txt") as f:
    lines = f.readlines()

cats = {}
for line in lines:
    line = line.strip()
    if not line.startswith("warning: "):
        continue
    msg = line[len("warning: "):]
    # Normalize backtick contents
    msg = re.sub(r"`[^`]*`", "X", msg)
    # Truncate
    if ": " in msg:
        msg = msg.split(": ")[0]
    cats[msg] = cats.get(msg, 0) + 1

for k, v in sorted(cats.items(), key=lambda x: -x[1]):
    print(f"{v:4d}  {k}")
