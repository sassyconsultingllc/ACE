"""Inject render_sandbox_panel method into app.rs before the closing } of impl BrowserApp."""
import re

with open(r"V:\sassy-browser-FIXED\src\app.rs", "r", encoding="utf-8") as f:
    content = f.read()

with open(r"V:\sassy-browser-FIXED\sandbox_panel_method.txt", "r", encoding="utf-8") as f:
    method = f.read()

# Find the pattern: closing brace of impl BrowserApp followed by impl eframe::App
target = "}\n\nimpl eframe::App for BrowserApp {"
if target not in content:
    # Try with different whitespace
    target = "}\r\n\r\nimpl eframe::App for BrowserApp {"

if target not in content:
    print("ERROR: Could not find injection point")
    # Try to find it with regex
    m = re.search(r'\}\s*\n\s*\n\s*impl eframe::App for BrowserApp \{', content)
    if m:
        print(f"Found at offset {m.start()}-{m.end()}")
        target = m.group(0)
    else:
        print("Still not found. Aborting.")
        exit(1)

replacement = "\n" + method + "\n}\n\nimpl eframe::App for BrowserApp {"
if "\r\n" in target:
    replacement = replacement.replace("\n", "\r\n")

content = content.replace(target, replacement, 1)

with open(r"V:\sassy-browser-FIXED\src\app.rs", "w", encoding="utf-8") as f:
    f.write(content)

print("SUCCESS: render_sandbox_panel method injected")
