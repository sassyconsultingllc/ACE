import re

with open('src/app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

with open('voice_panel_insert.txt', 'r', encoding='utf-8') as f:
    insert = f.read()

# Find the pattern: "fn protocol_url_encode..." followed by closing brace of impl block
# Insert voice panel method before the closing brace
target = '    fn protocol_url_encode(&self, input: &str) -> String {\n        url_encode(input)\n    }\n}'
replacement = '    fn protocol_url_encode(&self, input: &str) -> String {\n        url_encode(input)\n    }\n' + insert + '}\n'

if target not in content:
    print("ERROR: target pattern not found in app.rs")
    # Try to find what's there
    idx = content.find('fn protocol_url_encode')
    if idx >= 0:
        print(f"Found fn protocol_url_encode at offset {idx}")
        snippet = content[idx:idx+200]
        print(f"Context: {repr(snippet)}")
    exit(1)

count = content.count(target)
if count != 1:
    print(f"ERROR: found {count} occurrences of target, expected 1")
    exit(1)

content = content.replace(target, replacement, 1)

# Also add voice toggle button in toolbar - find the AI Assistant button line
toolbar_target = '            if self.svg_icons.button(ui, "robot", "AI Assistant").clicked() {\n                self.ai_sidebar_visible = !self.ai_sidebar_visible;\n            }'
toolbar_replacement = toolbar_target + '\n            if ui.button("Voice").on_hover_text("Voice Input Panel").clicked() {\n                self.voice_panel_visible = !self.voice_panel_visible;\n            }'

if toolbar_target in content:
    content = content.replace(toolbar_target, toolbar_replacement, 1)
    print("Added voice toggle button in toolbar")
else:
    print("WARNING: Could not find toolbar target for voice button")

# Add render_voice_panel() call in update() near other panel renders
panel_target = '        // History / activity panel\n        self.render_history_panel(ctx);'
panel_replacement = panel_target + '\n\n        // Voice input panel\n        self.render_voice_panel(ctx);'

if panel_target in content:
    content = content.replace(panel_target, panel_replacement, 1)
    print("Added render_voice_panel() call in update()")
else:
    print("WARNING: Could not find panel target for render_voice_panel call")

with open('src/app.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("SUCCESS: All edits applied")
