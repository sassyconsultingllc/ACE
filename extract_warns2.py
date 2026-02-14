"""Extract warnings grouped by file."""
import re
from collections import defaultdict

warnings_by_file = defaultdict(list)

with open("warn_current2.txt") as f:
    lines = f.readlines()

current_warning = ""
for i, line in enumerate(lines):
    if line.startswith("warning:"):
        current_warning = line.strip()
    elif line.strip().startswith("--> src"):
        m = re.match(r"\s*--> (src[^\s:]+)", line)
        if m:
            filepath = m.group(1).replace("\\", "/")
            warnings_by_file[filepath].append(current_warning)

# Print sorted by count
for filepath, warns in sorted(warnings_by_file.items(), key=lambda x: -len(x[1])):
    print(f"=== {filepath} ({len(warns)} warnings) ===")
