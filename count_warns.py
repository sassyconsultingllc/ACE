import re
from collections import Counter

c = Counter()
with open("warn_current.txt") as f:
    for line in f:
        if line.startswith("warning:"):
            cleaned = re.sub(r"`[^`]*`", "X", line.strip())
            cleaned = re.sub(r"\s+", " ", cleaned).strip()
            c[cleaned] += 1

for msg, count in c.most_common(20):
    print(f"{count:4d}  {msg}")
