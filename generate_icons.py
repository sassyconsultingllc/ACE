#!/usr/bin/env python3
"""Generate all SVG icon assets for sassy-browser."""
import os

OUTPUT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "assets", "icons", "svg")

def svg(inner):
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" '
        'width="24" height="24" fill="currentColor">'
        + inner + "</svg>"
    )

ICONS = {}

# Navigation / Transport
ICONS["arrow-left"] = svg('<polygon points="16,4 8,12 16,20"/>')
ICONS["arrow-right"] = svg('<polygon points="8,4 16,12 8,20"/>')
ICONS["skip-start"] = svg('<rect x="4" y="5" width="3" height="14"/><polygon points="18,5 9,12 18,19"/>')
ICONS["skip-end"] = svg('<polygon points="6,5 15,12 6,19"/><rect x="17" y="5" width="3" height="14"/>')
ICONS["play"] = svg('<polygon points="6,3 20,12 6,21"/>')
ICONS["pause"] = svg('<rect x="5" y="4" width="5" height="16"/><rect x="14" y="4" width="5" height="16"/>')
ICONS["stop"] = svg('<rect x="5" y="5" width="14" height="14" rx="1"/>')
ICONS["arrow-up"] = svg('<polygon points="12,4 4,20 20,20"/>')
ICONS["arrow-down"] = svg('<polygon points="4,4 20,4 12,20"/>')
ICONS["nav-back"] = svg('<path d="M15,4 L7,12 L15,20" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["nav-forward"] = svg('<path d="M9,4 L17,12 L9,20" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["reload"] = svg('<path d="M17.6,6.4A8,8,0,1,0,20,12" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/><polygon points="20,5 20,12 16,8"/>')
ICONS["close-x"] = svg('<path d="M6,6 L18,18 M18,6 L6,18" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>')

# Status / Info
ICONS["check"] = svg('<path d="M4,12 L9,18 L20,6" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["cross"] = svg('<path d="M6,6 L18,18 M18,6 L6,18" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>')
ICONS["warning"] = svg('<polygon points="12,2 1,21 23,21" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/><line x1="12" y1="10" x2="12" y2="15" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><circle cx="12" cy="18" r="1"/>')
ICONS["info"] = svg('<circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/><line x1="12" y1="11" x2="12" y2="17" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><circle cx="12" cy="8" r="1.2"/>')

# File types
ICONS["file-text"] = svg('<path d="M6,2 L14,2 L18,6 L18,22 L6,22 Z" fill="none" stroke="currentColor" stroke-width="1.5"/><polygon points="14,2 14,6 18,6"/><line x1="9" y1="10" x2="15" y2="10" stroke="currentColor" stroke-width="1.2"/><line x1="9" y1="13" x2="15" y2="13" stroke="currentColor" stroke-width="1.2"/><line x1="9" y1="16" x2="13" y2="16" stroke="currentColor" stroke-width="1.2"/>')
ICONS["file-code"] = svg('<path d="M6,2 L14,2 L18,6 L18,22 L6,22 Z" fill="none" stroke="currentColor" stroke-width="1.5"/><polygon points="14,2 14,6 18,6"/><path d="M10,11 L8,14 L10,17" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/><path d="M14,11 L16,14 L14,17" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["file-web"] = svg('<circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="1.5"/><ellipse cx="12" cy="12" rx="4" ry="10" fill="none" stroke="currentColor" stroke-width="1.5"/><line x1="2" y1="12" x2="22" y2="12" stroke="currentColor" stroke-width="1.5"/>')
ICONS["file-data"] = svg('<rect x="4" y="4" width="16" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="8" y="1" width="8" height="4" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><line x1="8" y1="11" x2="16" y2="11" stroke="currentColor" stroke-width="1.2"/><line x1="8" y1="14" x2="16" y2="14" stroke="currentColor" stroke-width="1.2"/>')
ICONS["file-image"] = svg('<rect x="3" y="3" width="18" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="8" cy="9" r="2"/><polygon points="5,19 10,13 13,16 16,11 19,19"/>')
ICONS["file-audio"] = svg('<circle cx="8" cy="18" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M11,18 L11,4 L20,2 L20,15" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><circle cx="17" cy="15" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/>')
ICONS["file-video"] = svg('<rect x="2" y="6" width="15" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><polygon points="19,9 23,7 23,17 19,15"/>')
ICONS["file-pdf"] = svg('<path d="M4,2 L4,22 L18,22 L18,6 L14,2 Z" fill="none" stroke="currentColor" stroke-width="1.5"/><polygon points="14,2 14,6 18,6"/><text x="7" y="17" font-size="8" font-family="sans-serif" font-weight="bold" fill="currentColor">PDF</text>')
ICONS["file-doc"] = svg('<path d="M6,2 L14,2 L18,6 L18,22 L6,22 Z" fill="none" stroke="currentColor" stroke-width="1.5"/><polygon points="14,2 14,6 18,6"/>')
ICONS["file-spreadsheet"] = svg('<rect x="2" y="3" width="20" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="5" y="13" width="3" height="5"/><rect x="10" y="9" width="3" height="9"/><rect x="15" y="6" width="3" height="12"/>')
ICONS["file-archive"] = svg('<rect x="3" y="8" width="18" height="14" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M1,8 L12,2 L23,8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><rect x="9" y="12" width="6" height="4" rx="1" fill="none" stroke="currentColor" stroke-width="1.2"/>')
ICONS["file-exe"] = svg('<circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M12,1 L12,4 M12,20 L12,23 M1,12 L4,12 M20,12 L23,12 M4.2,4.2 L6.3,6.3 M17.7,17.7 L19.8,19.8 M19.8,4.2 L17.7,6.3 M6.3,17.7 L4.2,19.8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>')
ICONS["folder-open"] = svg('<path d="M2,6 L2,20 L20,20 L22,10 L10,10 L8,6 Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>')
ICONS["folder-closed"] = svg('<path d="M2,4 L9,4 L11,7 L22,7 L22,20 L2,20 Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>')

# Actions
ICONS["extract"] = svg('<rect x="3" y="10" width="18" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M12,14 L12,2 M8,5 L12,1 L16,5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["add-file"] = svg('<path d="M6,2 L14,2 L18,6 L18,22 L6,22 Z" fill="none" stroke="currentColor" stroke-width="1.5"/><polygon points="14,2 14,6 18,6"/><line x1="12" y1="10" x2="12" y2="18" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><line x1="8" y1="14" x2="16" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>')
ICONS["plus"] = svg('<path d="M12,4 L12,20 M4,12 L20,12" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>')
ICONS["minus"] = svg('<line x1="4" y1="12" x2="20" y2="12" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>')
ICONS["search"] = svg('<circle cx="10" cy="10" r="7" fill="none" stroke="currentColor" stroke-width="2"/><line x1="15" y1="15" x2="21" y2="21" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>')
ICONS["tree-view"] = svg('<path d="M4,4 L4,20 M4,8 L10,8 M4,14 L10,14 M4,20 L10,20" fill="none" stroke="currentColor" stroke-width="1.3"/><rect x="12" y="6" width="8" height="4" fill="none" stroke="currentColor" stroke-width="1.3"/><rect x="12" y="12" width="8" height="4" fill="none" stroke="currentColor" stroke-width="1.3"/><rect x="12" y="18" width="8" height="4" fill="none" stroke="currentColor" stroke-width="1.3"/>')
ICONS["select-cursor"] = svg('<polygon points="5,2 5,18 10,14 15,20 17,19 12,13 18,12"/>')
ICONS["settings"] = svg('<circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M12,1 L12,4 M12,20 L12,23 M1,12 L4,12 M20,12 L23,12 M4.2,4.2 L6.3,6.3 M17.7,17.7 L19.8,19.8 M19.8,4.2 L17.7,6.3 M6.3,17.7 L4.2,19.8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>')
ICONS["lock"] = svg('<rect x="5" y="11" width="14" height="11" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M8,11 L8,7 A4,4,0,0,1,16,7 L16,11" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="12" cy="16" r="1.5"/>')
ICONS["user"] = svg('<circle cx="12" cy="8" r="4" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M4,22 C4,17 8,14 12,14 C16,14 20,17 20,22" fill="none" stroke="currentColor" stroke-width="1.5"/>')
ICONS["home"] = svg('<path d="M3,12 L12,3 L21,12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M5,12 L5,21 L19,21 L19,12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><rect x="10" y="15" width="4" height="6" fill="none" stroke="currentColor" stroke-width="1.2"/>')
ICONS["books"] = svg('<rect x="3" y="4" width="5" height="16" rx="1" fill="none" stroke="currentColor" stroke-width="1.3"/><rect x="10" y="6" width="5" height="14" rx="1" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M17,20 L17,3 L22,5 L22,22 Z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>')
ICONS["robot"] = svg('<rect x="4" y="8" width="16" height="13" rx="3" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="9" cy="14" r="2"/><circle cx="15" cy="14" r="2"/><line x1="10" y1="18" x2="14" y2="18" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><line x1="12" y1="4" x2="12" y2="8" stroke="currentColor" stroke-width="1.5"/><circle cx="12" cy="3" r="1.5"/>')
ICONS["shield"] = svg('<path d="M12,2 L3,6 L3,12 C3,17 7,21 12,22 C17,21 21,17 21,12 L21,6 Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>')
ICONS["blocked"] = svg('<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/><line x1="5.5" y1="5.5" x2="18.5" y2="18.5" stroke="currentColor" stroke-width="2"/>')
ICONS["fullscreen"] = svg('<path d="M3,9 L3,3 L9,3 M15,3 L21,3 L21,9 M21,15 L21,21 L15,21 M9,21 L3,21 L3,15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["exit-fullscreen"] = svg('<path d="M9,3 L9,9 L3,9 M15,9 L21,9 L15,3 M15,21 L15,15 L21,15 M9,15 L3,15 L9,21" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>')

# Audio
ICONS["volume-mute"] = svg('<polygon points="3,9 7,9 12,4 12,20 7,15 3,15"/><path d="M17,9 L21,15 M21,9 L17,15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>')
ICONS["volume-low"] = svg('<polygon points="3,9 7,9 12,4 12,20 7,15 3,15"/><path d="M16,9 C18,11 18,13 16,15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>')
ICONS["volume-high"] = svg('<polygon points="2,9 5,9 10,4 10,20 5,15 2,15"/><path d="M14,8 C16,10 16,14 14,16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><path d="M17,5 C20,8 20,16 17,19" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>')
ICONS["music-note"] = svg('<circle cx="7" cy="18" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="17" cy="16" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M10,18 L10,4 L20,2 L20,16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>')

# Status indicators
ICONS["circle-green"] = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24"><circle cx="12" cy="12" r="8" fill="#4CAF50"/></svg>'
ICONS["circle-gray"] = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24"><circle cx="12" cy="12" r="8" fill="#9E9E9E"/></svg>'
ICONS["download"] = svg('<path d="M12,3 L12,17 M6,13 L12,19 L18,13" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/><line x1="4" y1="22" x2="20" y2="22" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>')
ICONS["upload"] = svg('<path d="M12,21 L12,7 M6,11 L12,5 L18,11" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/><line x1="4" y1="2" x2="20" y2="2" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>')

# Misc
ICONS["bullet"] = svg('<circle cx="12" cy="12" r="4"/>')
ICONS["summation"] = svg('<path d="M18,4 L6,4 L13,12 L6,20 L18,20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["grid"] = svg('<rect x="3" y="3" width="18" height="18" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><line x1="9" y1="3" x2="9" y2="21" stroke="currentColor" stroke-width="1.5"/><line x1="15" y1="3" x2="15" y2="21" stroke="currentColor" stroke-width="1.5"/><line x1="3" y1="9" x2="21" y2="9" stroke="currentColor" stroke-width="1.5"/><line x1="3" y1="15" x2="21" y2="15" stroke="currentColor" stroke-width="1.5"/>')
ICONS["pilcrow"] = svg('<path d="M10,4 C7,4 5,6 5,8.5 C5,11 7,13 10,13 L10,22 M14,4 L14,22 M10,4 L18,4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>')
ICONS["pencil"] = svg('<path d="M4,20 L4,16 L16,4 L20,8 L8,20 Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><line x1="14" y1="6" x2="18" y2="10" stroke="currentColor" stroke-width="1.5"/>')

# Write all
os.makedirs(OUTPUT_DIR, exist_ok=True)
count = 0
for name, content in sorted(ICONS.items()):
    path = os.path.join(OUTPUT_DIR, f"{name}.svg")
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    count += 1

print(f"Generated {count} SVG icons in {OUTPUT_DIR}")
