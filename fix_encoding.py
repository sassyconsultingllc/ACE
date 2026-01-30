#!/usr/bin/env python3
import os
import re

def fix_app_rs():
    path = r'V:\sassy-browser-FIXED\src\app.rs'
    with open(path, 'rb') as f:
        content = f.read()
    
    original = content
    
    # Fix the back button (line ~573)
    content = re.sub(
        rb'else if ui\.add_enabled\(can_back, egui::Button::new\("[^"]*"\)\.min_size',
        b'else if ui.add_enabled(can_back, egui::Button::new("<").min_size',
        content
    )
    
    # Fix the forward button (line ~585)
    content = re.sub(
        rb'else if ui\.add_enabled\(can_forward, egui::Button::new\("[^"]*"\)\.min_size',
        b'else if ui.add_enabled(can_forward, egui::Button::new(">").min_size',
        content
    )
    
    # Fix the stop button
    content = re.sub(
        rb'if ui\.button\("[^"]*"\)\.on_hover_text\("Stop"\)',
        b'if ui.button("X").on_hover_text("Stop")',
        content
    )
    
    # Fix the reload button
    content = re.sub(
        rb'if ui\.button\("[^"]*"\)\.on_hover_text\("Reload',
        b'if ui.button("R").on_hover_text("Reload',
        content
    )
    
    if content != original:
        with open(path, 'wb') as f:
            f.write(content)
        print(f'Fixed app.rs')
    else:
        print('No changes to app.rs')

def fix_all_emoji_strings():
    """Fix common mojibake patterns across all files"""
    src_dir = r'V:\sassy-browser-FIXED\src'
    
    # Mapping of mojibake to ASCII alternatives
    # These are UTF-8 bytes that got double-encoded
    replacements = [
        # Bullets and separators
        (rb'\xe2\x80\xa2', b'|'),  # bullet
        (rb'\xc3\xa2\xe2\x82\xac\xc2\xa2', b'|'),  # double-encoded bullet
        
        # Arrows
        (rb'\xe2\x86\x90', b'<'),  # left arrow
        (rb'\xe2\x86\x92', b'>'),  # right arrow  
        (rb'\xe2\x86\xbb', b'R'),  # reload symbol
        (rb'\xe2\x97\x80', b'<'),  # triangle left
        (rb'\xe2\x96\xb6', b'>'),  # triangle right
        
        # X marks
        (rb'\xe2\x9c\x95', b'X'),  # multiplication X
        (rb'\xe2\x9c\x96', b'X'),  # heavy X
        (rb'\xc3\x97', b'X'),  # multiplication sign
    ]
    
    for root, dirs, files in os.walk(src_dir):
        for f in files:
            if f.endswith('.rs'):
                path = os.path.join(root, f)
                with open(path, 'rb') as fp:
                    content = fp.read()
                
                original = content
                for pattern, replacement in replacements:
                    content = content.replace(pattern, replacement)
                
                if content != original:
                    with open(path, 'wb') as fp:
                        fp.write(content)
                    print(f'Fixed: {path}')

if __name__ == '__main__':
    fix_app_rs()
    fix_all_emoji_strings()
    print('Done!')
