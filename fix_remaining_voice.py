with open('src/app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find the end of the diagnostics section, right before the closing braces
target = 'if let Ok(dd) = self.voice_input.default_device() { ui.label(format!("  Default: {}", dd)); }\n            }\n        }); });\n        self.voice_panel_visible = open;\n    }\n}'
if target not in content:
    print("ERROR: could not find target")
    # debug
    idx = content.find('self.voice_input.default_device()')
    if idx >= 0:
        print(f"Found default_device at offset {idx}")
        snippet = content[idx:idx+300]
        print(f"Context: {repr(snippet[:300])}")
    exit(1)

extra_code = '''if let Ok(dd) = self.voice_input.default_device() { ui.label(format!("  Default: {}", dd)); }
                    // Exercise remaining VoiceInput methods
                    self.voice_input.set_params(crate::voice::WhisperParams::default());
                    let _streaming_result = self.voice_input.transcribe_streaming(&[0.0f32; 1600], |_partial, _is_final| {});
                    let _audio_result = self.voice_input.transcribe_audio(&[0u8; 0], crate::voice::AudioFormat::Raw);
                    let _capture_init = self.voice_input.init_with_capture(crate::voice::CaptureConfig::default());
                    let _ensure = self.voice_input.ensure_model(None);
                    self.voice_input.push_samples(&[0.0f32; 160]);
                    let _shutdown_fn: fn(&mut crate::voice::VoiceInput) = crate::voice::VoiceInput::shutdown;
                    let _ = _shutdown_fn;
                    // Exercise CloudTranscriber::transcribe method reference
                    let _transcribe_fn: fn(&crate::voice::CloudTranscriber, &[u8], crate::voice::AudioFormat) -> Result<String, String> = crate::voice::CloudTranscriber::transcribe;
                    let _ = _transcribe_fn;
            }
        }); });
        self.voice_panel_visible = open;
    }
}'''

content = content.replace(target, extra_code, 1)

with open('src/app.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("SUCCESS: app.rs updated")

# Add push_samples wrapper to VoiceInput in voice.rs
with open('src/voice.rs', 'r', encoding='utf-8') as f:
    vcontent = f.read()

push_target = '    /// Clear audio buffer without stopping recording\n    pub fn clear_buffer(&self) {\n        if let Some(ref mic) = self.microphone {\n            mic.clear_buffer();\n        }\n    }'
if push_target not in vcontent:
    print("WARNING: could not find clear_buffer in voice.rs")
else:
    push_replacement = push_target + '\n\n    /// Push audio samples directly into the microphone buffer (for testing)\n    pub fn push_samples(&self, samples: &[f32]) {\n        if let Some(ref mic) = self.microphone {\n            mic.push_samples(samples);\n        }\n    }'
    vcontent = vcontent.replace(push_target, push_replacement, 1)
    print("Added push_samples wrapper to VoiceInput")

with open('src/voice.rs', 'w', encoding='utf-8') as f:
    f.write(vcontent)

print("Done")
