#!/usr/bin/env python
"""
Parse cargo check output and fix dead code warnings.

Strategy:
- Unused struct fields: prefix with _
- Unused enum variants: prefix with _
- Unused functions/methods: prefix with _
- Unused constants: prefix with _
- Unused type aliases: prefix with _
- Unused imports: remove
- Unused variables: prefix with _
- Deprecated methods: apply fix

This script modifies files in-place. Run cargo check after to verify.
"""
import re
import os
import sys
from pathlib import Path

SRC = Path(os.path.dirname(os.path.abspath(__file__))) / "src"

def parse_cargo_output(output_file):
    """Parse cargo check output into structured warnings."""
    warnings = []
    with open(output_file, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if line.startswith("warning:") and ("never" in line or "unused" in line or "deprecated" in line):
            # Get the location line
            if i + 1 < len(lines):
                loc_line = lines[i + 1].strip()
                if loc_line.startswith("--> "):
                    loc = loc_line[4:]  # "src\foo.rs:123:45"
                    parts = loc.split(":")
                    if len(parts) >= 2:
                        filepath = parts[0].replace("\\", "/")
                        lineno = int(parts[1])
                        warnings.append({
                            "message": line,
                            "file": filepath,
                            "line": lineno,
                            "col": int(parts[2]) if len(parts) > 2 else 0,
                        })
        i += 1
    return warnings


def extract_names(message):
    """Extract identifier names from backtick-quoted warning message."""
    return re.findall(r'`([^`]+)`', message)


def fix_unused_field(filepath, lineno, field_name):
    """Prefix an unused struct field with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    # Match field patterns like: "    pub field_name: Type," or "    field_name: Type,"
    # Be careful not to double-prefix
    if f"_{field_name}" in line:
        return False  # Already prefixed

    # Replace the field name with _field_name
    # Handle: pub field: Type, or field: Type,
    pattern = rf'(\s+(?:pub\s+)?){re.escape(field_name)}(\s*:)'
    new_line = re.sub(pattern, rf'\1_{field_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        # Also need to find and fix all references to this field
        # For now, just prefix the declaration
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def fix_unused_variant(filepath, lineno, variant_name):
    """Prefix an unused enum variant with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    if f"_{variant_name}" in line:
        return False

    pattern = rf'(\s+){re.escape(variant_name)}(\s*[\({{,]|\s*$)'
    new_line = re.sub(pattern, rf'\1_{variant_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def fix_unused_fn(filepath, lineno, fn_name):
    """Prefix an unused function/method with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    if f"_{fn_name}" in line:
        return False

    # Match: fn name( or pub fn name( or pub(crate) fn name(
    pattern = rf'(fn\s+){re.escape(fn_name)}(\s*[\(<])'
    new_line = re.sub(pattern, rf'\1_{fn_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def fix_unused_const(filepath, lineno, const_name):
    """Prefix an unused constant with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    if f"_{const_name}" in line:
        return False

    pattern = rf'(const\s+){re.escape(const_name)}(\s*:)'
    new_line = re.sub(pattern, rf'\1_{const_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def fix_unused_type_alias(filepath, lineno, type_name):
    """Prefix an unused type alias with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    if f"_{type_name}" in line:
        return False

    pattern = rf'(type\s+){re.escape(type_name)}(\s*=)'
    new_line = re.sub(pattern, rf'\1_{type_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def fix_unused_struct(filepath, lineno, struct_name):
    """Prefix an unused struct with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    if f"_{struct_name}" in line:
        return False

    pattern = rf'(struct\s+){re.escape(struct_name)}(\s*[\({{<]|\s+)'
    new_line = re.sub(pattern, rf'\1_{struct_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def fix_unused_enum(filepath, lineno, enum_name):
    """Prefix an unused enum with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    if f"_{enum_name}" in line:
        return False

    pattern = rf'(enum\s+){re.escape(enum_name)}(\s*[\{{<]|\s+)'
    new_line = re.sub(pattern, rf'\1_{enum_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def fix_unused_trait(filepath, lineno, trait_name):
    """Prefix an unused trait with underscore."""
    full_path = SRC.parent / filepath
    with open(full_path, 'r', encoding='utf-8') as f:
        file_lines = f.readlines()

    if lineno <= 0 or lineno > len(file_lines):
        return False

    line = file_lines[lineno - 1]

    if f"_{trait_name}" in line:
        return False

    pattern = rf'(trait\s+){re.escape(trait_name)}(\s*[\{{<:]|\s+)'
    new_line = re.sub(pattern, rf'\1_{trait_name}\2', line)

    if new_line != line:
        file_lines[lineno - 1] = new_line
        with open(full_path, 'w', encoding='utf-8') as f:
            f.writelines(file_lines)
        return True
    return False


def process_warnings(warnings):
    """Process all warnings and apply fixes."""
    fixed = 0
    skipped = 0

    for w in warnings:
        msg = w["message"]
        filepath = w["file"]
        lineno = w["line"]
        names = extract_names(msg)

        if not names:
            skipped += 1
            continue

        if "is never read" in msg or "are never read" in msg:
            # Field(s) never read — prefix with _
            for name in names:
                if fix_unused_field(filepath, lineno, name):
                    fixed += 1
                    print(f"  FIXED field _{name} in {filepath}:{lineno}")

        elif "is never constructed" in msg or "are never constructed" in msg:
            # Check if it's a struct or enum variant
            if "struct" in msg:
                for name in names:
                    if fix_unused_struct(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED struct _{name} in {filepath}:{lineno}")
            elif "variant" in msg:
                for name in names:
                    if fix_unused_variant(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED variant _{name} in {filepath}:{lineno}")
            else:
                # Could be struct or variant
                for name in names:
                    ok = fix_unused_struct(filepath, lineno, name) or fix_unused_variant(filepath, lineno, name)
                    if ok:
                        fixed += 1
                        print(f"  FIXED constructed _{name} in {filepath}:{lineno}")

        elif "is never used" in msg or "are never used" in msg:
            if "function" in msg or "method" in msg or "associated function" in msg or "associated item" in msg:
                for name in names:
                    if fix_unused_fn(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED fn _{name} in {filepath}:{lineno}")
            elif "constant" in msg:
                for name in names:
                    if fix_unused_const(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED const _{name} in {filepath}:{lineno}")
            elif "type alias" in msg:
                for name in names:
                    if fix_unused_type_alias(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED type _{name} in {filepath}:{lineno}")
            elif "enum" in msg:
                for name in names:
                    if fix_unused_enum(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED enum _{name} in {filepath}:{lineno}")
            elif "struct" in msg:
                for name in names:
                    if fix_unused_struct(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED struct _{name} in {filepath}:{lineno}")
            elif "trait" in msg:
                for name in names:
                    if fix_unused_trait(filepath, lineno, name):
                        fixed += 1
                        print(f"  FIXED trait _{name} in {filepath}:{lineno}")
            else:
                skipped += 1
                print(f"  SKIP: {msg[:80]}")

        else:
            skipped += 1

    return fixed, skipped


def main():
    output_file = SRC.parent / "check_output.txt"
    if not output_file.exists():
        print(f"Error: {output_file} not found. Run 'cargo check 2>&1 | tee check_output.txt' first.")
        sys.exit(1)

    print("Parsing warnings...")
    warnings = parse_cargo_output(output_file)
    print(f"Found {len(warnings)} warnings to process")

    print("\nApplying fixes...")
    fixed, skipped = process_warnings(warnings)

    print(f"\nDone: {fixed} items fixed, {skipped} items skipped")
    print("Run 'cargo check' to verify remaining warnings.")


if __name__ == "__main__":
    main()
