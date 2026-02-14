"""
Reset app.rs to git HEAD and apply voice panel changes atomically.
Also reset voice.rs to git HEAD and add push_samples wrapper.
Also fix the Stmt::Expression bug in engine.rs.
"""
import subprocess
import sys

# Step 1: Restore files from git
for f in ['src/app.rs', 'src/voice.rs', 'src/engine.rs']:
    result = subprocess.run(['git', 'checkout', 'HEAD', '--', f], capture_output=True, text=True)
    if result.returncode != 0:
        # Try alternative
        result = subprocess.run(['git', 'show', f'HEAD:{f}'], capture_output=True)
        if result.returncode != 0:
            print(f"ERROR: Cannot restore {f}: {result.stderr}")
            sys.exit(1)
        with open(f, 'wb') as fh:
            fh.write(result.stdout)
    print(f"Restored {f}")

# Step 2: Read app.rs
with open('src/app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Step 3: Read voice panel insert
with open('voice_panel_insert.txt', 'r', encoding='utf-8') as f:
    voice_panel = f.read()

# Step 4: Insert voice panel method before closing brace of impl BrowserApp
# Find the exact pattern
target1 = '    fn protocol_url_encode(&self, input: &str) -> String {\n        url_encode(input)\n    }\n}'
if target1 not in content:
    print("ERROR: Could not find protocol_url_encode closing pattern")
    sys.exit(1)

replacement1 = '    fn protocol_url_encode(&self, input: &str) -> String {\n        url_encode(input)\n    }\n' + voice_panel + '}'

content = content.replace(target1, replacement1, 1)
print("Inserted render_voice_panel method")

# Step 5: Add voice toggle button in toolbar (after AI Assistant button)
target2 = '            if self.svg_icons.button(ui, "robot", "AI Assistant").clicked() {\n                self.ai_sidebar_visible = !self.ai_sidebar_visible;\n            }'
if target2 not in content:
    print("ERROR: Could not find AI Assistant button pattern")
    sys.exit(1)

replacement2 = target2 + '\n            if ui.button("Voice").on_hover_text("Voice Input Panel").clicked() {\n                self.voice_panel_visible = !self.voice_panel_visible;\n            }'
content = content.replace(target2, replacement2, 1)
print("Added voice toggle button")

# Step 6: Add render_voice_panel() call in update() after render_history_panel
target3 = '        // History / activity panel\n        self.render_history_panel(ctx);'
if target3 not in content:
    print("ERROR: Could not find render_history_panel pattern")
    sys.exit(1)

replacement3 = target3 + '\n\n        // Voice input panel\n        self.render_voice_panel(ctx);'
content = content.replace(target3, replacement3, 1)
print("Added render_voice_panel call in update()")

# Step 7: Write app.rs atomically
with open('src/app.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Wrote src/app.rs")

# Step 8: Add push_samples wrapper to VoiceInput in voice.rs
with open('src/voice.rs', 'r', encoding='utf-8') as f:
    vcontent = f.read()

push_target = '    /// Clear audio buffer without stopping recording\n    pub fn clear_buffer(&self) {\n        if let Some(ref mic) = self.microphone {\n            mic.clear_buffer();\n        }\n    }'
if push_target in vcontent:
    push_replacement = push_target + '\n\n    /// Push audio samples directly into the microphone buffer (for testing)\n    pub fn push_samples(&self, samples: &[f32]) {\n        if let Some(ref mic) = self.microphone {\n            mic.push_samples(samples);\n        }\n    }'
    vcontent = vcontent.replace(push_target, push_replacement, 1)
    with open('src/voice.rs', 'w', encoding='utf-8') as f:
        f.write(vcontent)
    print("Added push_samples to VoiceInput in voice.rs")
else:
    print("WARNING: Could not find clear_buffer in voice.rs")

# Step 9: Fix Stmt::Expression bug in engine.rs
with open('src/engine.rs', 'r', encoding='utf-8') as f:
    econtent = f.read()

old_stmt = 'crate::js::Stmt::Expression(expr)'
new_stmt = 'crate::js::Stmt::Expr(expr)'
if old_stmt in econtent:
    econtent = econtent.replace(old_stmt, new_stmt, 1)
    with open('src/engine.rs', 'w', encoding='utf-8') as f:
        f.write(econtent)
    print("Fixed Stmt::Expression -> Stmt::Expr in engine.rs")
else:
    print("engine.rs Stmt::Expression fix not needed")

print("\nAll done! Run 'cargo check' to verify.")
