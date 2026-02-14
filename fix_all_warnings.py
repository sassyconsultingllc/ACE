#!/usr/bin/env python
"""
Fix ALL cargo warnings properly:
1. Add #![allow(dead_code)] to WIP module files (doesn't rename anything)
2. Fix deprecated method calls
3. Fix unused variables/imports/assignments
"""
import re
import os
from pathlib import Path

SRC = Path(os.path.dirname(os.path.abspath(__file__))) / "src"

# ============================================================================
# STEP 1: Files that need #![allow(dead_code)] at the top
# These are WIP modules with unused structs/enums/functions/fields
# ============================================================================
DEAD_CODE_FILES = [
    "src/adblock.rs",
    "src/ai.rs",
    "src/console.rs",
    "src/cookies.rs",
    "src/crypto.rs",
    "src/data.rs",
    "src/dom.rs",
    "src/engine.rs",
    "src/extensions.rs",
    "src/fontcase.rs",
    "src/hittest.rs",
    "src/icons.rs",
    "src/imaging.rs",
    "src/js/lexer.rs",
    "src/json_viewer.rs",
    "src/layout.rs",
    "src/layout_engine.rs",
    "src/markdown.rs",
    "src/mcp.rs",
    "src/mcp_api.rs",
    "src/mcp_fs.rs",
    "src/mcp_git.rs",
    "src/mcp_panel.rs",
    "src/mcp_server.rs",
    "src/network.rs",
    "src/paint.rs",
    "src/print.rs",
    "src/protocol.rs",
    "src/renderer.rs",
    "src/rest_client.rs",
    "src/sandbox/mod.rs",
    "src/sandbox/network.rs",
    "src/sandbox/page.rs",
    "src/sandbox/popup.rs",
    "src/sandbox/quarantine.rs",
    "src/script_engine.rs",
    "src/setup.rs",
    "src/sync/family.rs",
    "src/sync/protocol.rs",
    "src/sync/secure.rs",
    "src/sync/server.rs",
    "src/sync/users.rs",
    "src/syntax.rs",
    "src/ui/app.rs",
    "src/ui/input.rs",
    "src/ui/mod.rs",
    "src/ui/network_bar.rs",
    "src/ui/popup.rs",
    "src/ui/render.rs",
    "src/ui/sidebar.rs",
    "src/ui/tabs.rs",
    "src/ui/theme.rs",
    "src/update.rs",
    "src/viewers/pdf.rs",
    "src/voice.rs",
]


def add_allow_dead_code(filepath):
    """Add #![allow(dead_code)] at the top of a file if not already present."""
    full_path = SRC.parent / filepath
    if not full_path.exists():
        return False

    with open(full_path, 'r', encoding='utf-8') as f:
        content = f.read()

    if '#![allow(dead_code)]' in content or '#[allow(dead_code)]' in content[:500]:
        return False

    # Insert after any existing #![...] attributes or //! doc comments at the top
    lines = content.split('\n')
    insert_idx = 0

    # Skip BOM
    if lines and lines[0].startswith('\ufeff'):
        lines[0] = lines[0][1:]

    # Skip leading //! doc comments and #![...] attributes
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith('//!') or stripped.startswith('#![') or stripped == '':
            insert_idx = i + 1
        else:
            break

    # If we're at the very start, insert after any initial comments
    if insert_idx == 0:
        # Check for // comments at top
        for i, line in enumerate(lines):
            stripped = line.strip()
            if stripped.startswith('//') or stripped == '':
                insert_idx = i + 1
            else:
                break

    lines.insert(insert_idx, '#![allow(dead_code)]')
    lines.insert(insert_idx + 1, '')

    with open(full_path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))
    return True


# ============================================================================
# STEP 2: Specific targeted fixes
# ============================================================================
def fix_file(filepath, replacements):
    """Apply specific string replacements in a file."""
    full_path = SRC.parent / filepath
    if not full_path.exists():
        return 0

    with open(full_path, 'r', encoding='utf-8') as f:
        content = f.read()

    count = 0
    for old, new in replacements:
        if old in content:
            content = content.replace(old, new)
            count += 1

    if count > 0:
        with open(full_path, 'w', encoding='utf-8') as f:
            f.write(content)
    return count


def main():
    print("=" * 60)
    print("STEP 1: Adding #![allow(dead_code)] to WIP module files")
    print("=" * 60)

    added = 0
    for f in DEAD_CODE_FILES:
        if add_allow_dead_code(f):
            added += 1
            print(f"  + {f}")
    print(f"  Added to {added} files\n")

    print("=" * 60)
    print("STEP 2: Fixing deprecated method calls")
    print("=" * 60)

    # Fix clamp_range -> range in print.rs
    n = fix_file("src/print.rs", [
        ('.clamp_range(', '.range('),
    ])
    print(f"  print.rs: {n} deprecated fixes\n")

    print("=" * 60)
    print("STEP 3: Fixing unused variables and imports")
    print("=" * 60)

    # adblock.rs: unused import, unused variables
    n = fix_file("src/adblock.rs", [
        # Remove unused import
        ('    pub fn render(&mut self, ui: &mut eframe::egui::Ui) {\n        use eframe::egui;\n        \n        ui.heading',
         '    pub fn render(&mut self, ui: &mut eframe::egui::Ui) {\n        ui.heading'),
        # Unused variables
        ('fn parse_cosmetic_exception(&self, line: &str)',
         'fn parse_cosmetic_exception(&self, _line: &str)'),
        ('for rule in &self.custom_rules {\n            // Same logic as above\n        }',
         'for _rule in &self.custom_rules {\n            // Same logic as above\n        }'),
        ('request_domain: &str,\n        resource_type: ResourceType,',
         '_request_domain: &str,\n        resource_type: ResourceType,'),
    ])
    print(f"  adblock.rs: {n} fixes")

    # mcp_server.rs: unused variable
    n = fix_file("src/mcp_server.rs", [
        ('McpCommand::TypeText { tab_id: _, text, element_ref, clear_first } => {\n            // If element_ref provided, find the form input in the DOM and set its value\n            if let Some(ref eref) = element_ref {',
         'McpCommand::TypeText { tab_id: _, text, element_ref, clear_first } => {\n            // If element_ref provided, find the form input in the DOM and set its value\n            if let Some(ref _eref) = element_ref {'),
    ])
    print(f"  mcp_server.rs: {n} fixes")

    # pdf.rs: unused assignments
    n = fix_file("src/viewers/pdf.rs", [
        ('let mut path_start_x: f32 = 0.0;\n        let mut path_start_y: f32 = 0.0;',
         'let mut _path_start_x: f32 = 0.0;\n        let mut _path_start_y: f32 = 0.0;'),
        ('path_start_x = current_x;\n                        path_start_y = current_y;',
         '_path_start_x = current_x;\n                        _path_start_y = current_y;'),
    ])
    print(f"  viewers/pdf.rs: {n} fixes")

    # icons.rs: fix text_button to use icon_name
    n = fix_file("src/icons.rs", [
        ('    pub fn text_button(\n        &self,\n        ui: &mut egui::Ui,\n        icon_name: &str,\n        text: &str,\n        tooltip: &str,\n    ) -> egui::Response {\n        // We use a horizontal layout inside a button-like frame\n        let btn = egui::Button::new({\n            let mut job = egui::text::LayoutJob::default();\n            // We can\'t easily embed an image in a Button label in egui,\n            // so we use the text-only path and prepend a space for the icon.\n            // The icon will be rendered separately via ui.horizontal.\n            job.append(text, 0.0, egui::TextFormat::default());\n            job\n        });\n        // For now, just return text button \xe2\x80\x94 icon rendered adjacently by caller\n        ui.button(text).on_hover_text(tooltip)\n    }',
         '    pub fn text_button(\n        &self,\n        ui: &mut egui::Ui,\n        icon_name: &str,\n        text: &str,\n        tooltip: &str,\n    ) -> egui::Response {\n        // Render icon inline before the text button\n        if let Some(tex) = self.textures.get(icon_name) {\n            let text_height = ui.text_style_height(&egui::TextStyle::Body);\n            ui.image((tex.id(), Vec2::splat(text_height)));\n        }\n        ui.button(text).on_hover_text(tooltip)\n    }'),
    ])
    print(f"  icons.rs: {n} fixes")

    # print.rs: unused assignment
    n = fix_file("src/print.rs", [
        ('    let mut current_page = page1;\n    let mut current_layer_ref = current_layer;',
         '    #[allow(unused_assignments)]\n    let mut current_page = page1;\n    let mut current_layer_ref = current_layer;'),
    ])
    print(f"  print.rs (unused assign): {n} fixes")

    print("\n" + "=" * 60)
    print("DONE! Run 'cargo check' to verify.")
    print("=" * 60)


if __name__ == "__main__":
    main()
