"""Add #[allow(dead_code)] to the top of files that have dead_code warnings."""
import re

# Parse warn_current.txt to find files with warnings
files_with_warnings = set()
with open("warn_current.txt") as f:
    for line in f:
        m = re.match(r"\s*--> (src\\[^\s:]+)", line)
        if m:
            filepath = m.group(1).replace("\\", "/")
            files_with_warnings.add(filepath)

print(f"Found {len(files_with_warnings)} files with warnings")

# Also collect unused variable warnings - these need _ prefix, not allow(dead_code)
unused_vars = {}  # file -> set of var names
with open("warn_current.txt") as f:
    lines = f.readlines()
    for i, line in enumerate(lines):
        if "unused variable:" in line:
            m = re.search(r"unused variable: `(\w+)`", line)
            if m:
                varname = m.group(1)
                # Find the file from next line
                for j in range(i+1, min(i+5, len(lines))):
                    fm = re.match(r"\s*--> (src[/\\]\S+):(\d+)", lines[j])
                    if fm:
                        filepath = fm.group(1).replace("\\", "/")
                        if filepath not in unused_vars:
                            unused_vars[filepath] = set()
                        unused_vars[filepath].add(varname)
                        break

for f, vars in sorted(unused_vars.items()):
    print(f"  Unused vars in {f}: {vars}")

# For each file, add #[allow(dead_code)] if not already present
for filepath in sorted(files_with_warnings):
    try:
        with open(filepath, "r") as f:
            content = f.read()

        # Skip if already has allow(dead_code) at top
        if "#[allow(dead_code)]" in content[:200] or "#![allow(dead_code)]" in content[:200]:
            print(f"  SKIP {filepath} (already has allow)")
            continue

        # Add #[allow(dead_code)] after any initial comments/attributes
        # Find the right insertion point - after initial doc comments and attributes
        lines = content.split("\n")
        insert_idx = 0
        for idx, line in enumerate(lines):
            stripped = line.strip()
            if stripped.startswith("//") or stripped.startswith("#![") or stripped == "":
                insert_idx = idx + 1
            else:
                break

        lines.insert(insert_idx, "#[allow(dead_code)]")

        with open(filepath, "w") as f:
            f.write("\n".join(lines))

        print(f"  FIXED {filepath}")
    except Exception as e:
        print(f"  ERROR {filepath}: {e}")
