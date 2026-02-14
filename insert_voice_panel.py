"""Insert voice panel method into app.rs and add the call in update()."""
import re

with open(r'V:\sassy-browser-FIXED\src\app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Read the method to insert
with open(r'V:\sassy-browser-FIXED\voice_panel_method.txt', 'r', encoding='utf-8') as f:
    method_text = f.read()

# 1. Add voice imports after the adblock import line (if not already present)
if 'use crate::voice::' not in content:
    import_line = 'use crate::adblock::{AdBlocker, AdBlockerUI, ResourceType as AdResourceType};'
    voice_import = """use crate::voice::{
    VoiceConfig, VoiceState, VoiceSession, WhisperModel, WhisperEngine,
    TranscriptResult, TranscriptSegment, AudioFormat, AudioDevice,
    MicrophoneCapture, VoiceActivityDetector, VoiceInput, VoiceCommandResult,
    VoiceCommand, CloudProvider, CloudTranscriber, HotkeyConfig, HotkeyModifiers,
    TriggerMode, list_audio_devices, key_name,
};"""
    content = content.replace(import_line, import_line + '\n' + voice_import)
    print("Added voice imports")
else:
    print("Voice imports already present")

# 2. Insert the render_voice_panel method before the closing } of impl BrowserApp
# Find "impl eframe::App for BrowserApp {" and insert the method before the } that precedes it
marker = 'impl eframe::App for BrowserApp {'
idx = content.find(marker)
if idx == -1:
    print("ERROR: Could not find 'impl eframe::App for BrowserApp {'")
    exit(1)

# Find the closing } of impl BrowserApp - it should be the } just before the marker
# Go backwards from marker to find the }
search_area = content[:idx].rstrip()
if search_area.endswith('}'):
    # Insert the method before this closing }
    insert_pos = search_area.rfind('}')
    if 'render_voice_panel' not in content:
        content = content[:insert_pos] + '\n' + method_text + '\n' + content[insert_pos:]
        print("Inserted render_voice_panel method")
    else:
        print("render_voice_panel already present")
else:
    print("ERROR: Could not find closing } of impl BrowserApp")
    exit(1)

# 3. Add the call self.render_voice_panel(ctx); in the update() method
# Find it near other render_*_panel calls
if 'self.render_voice_panel(ctx)' not in content:
    # Insert after render_history_panel call
    history_call = 'self.render_history_panel(ctx);'
    voice_call = '\n        // Voice input panel\n        self.render_voice_panel(ctx);'
    content = content.replace(history_call, history_call + voice_call, 1)
    print("Added render_voice_panel call in update()")
else:
    print("render_voice_panel call already present")

with open(r'V:\sassy-browser-FIXED\src\app.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Done!")
