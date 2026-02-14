with open('src/app.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old = 'self.render_threat_protection_panel(ui);'
new = 'ui.label("Threat protection panel (placeholder)");'

if old in content:
    content = content.replace(old, new, 1)
    with open('src/app.rs', 'w', encoding='utf-8') as f:
        f.write(content)
    print("Fixed threat_protection_panel")
else:
    print("Target not found")
