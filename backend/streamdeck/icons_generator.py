import subprocess
from pathlib import Path

ICONS_DIR = Path("/home/linux/.var/app/com.core447.StreamController/data/custom_icons")
WORKSPACE_FRONTEND = Path("/home/linux/.gemini/antigravity-ide/scratch/algodeck/frontend/icons")

def generate_icons():
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    icons = {
        "icon_test_sec": {
            "label": "TESTUJ",
            "stroke": "#10b981", # Emerald
            "glyph": """
                <g transform="translate(72, 54)">
                    <path d="M 0,-24 C 8,-16 12,-4 12,12 L -12,12 C -12,-4 -8,-16 0,-24 Z" fill="#10b981"/>
                    <circle cx="0" cy="-6" r="4.5" fill="#10121b"/>
                    <circle cx="0" cy="-6" r="2.5" fill="#34d399"/>
                    <path d="M -12,4 L -20,16 L -12,14 Z" fill="#059669"/>
                    <path d="M 12,4 L 20,16 L 12,14 Z" fill="#059669"/>
                    <path d="M -6,14 Q 0,26 0,26 Q 0,26 6,14 Z" fill="#f59e0b"/>
                    <path d="M -3,14 Q 0,21 0,21 Q 0,21 3,14 Z" fill="#fef08a"/>
                </g>
            """
        },
        "icon_play_sec": {
            "label": "ODPAL",
            "stroke": "#06b6d4", # Cyan
            "glyph": """
                <g transform="translate(72, 54)">
                    <path d="M -12,-18 C -12,-20 -9.5,-21.5 -7.5,-20.2 L 18,-4 C 19.8,-2.8 19.8,0.2 18,1.4 L -7.5,17.6 C -9.5,18.9 -12,17.4 -12,15.4 Z" fill="#06b6d4"/>
                </g>
            """
        },
        "icon_kill_sec": {
            "label": "KILL",
            "stroke": "#f43f5e", # Rose / Red
            "glyph": """
                <g transform="translate(72, 54)">
                    <path d="M 2,-22 L -16,-2 L -3,-2 L -6,22 L 16,2 L 3,2 Z" fill="#f43f5e"/>
                </g>
            """
        },
        "icon_task_sec": {
            "label": "ZADANIA",
            "stroke": "#a855f7", # Purple
            "glyph": """
                <g transform="translate(72, 54)">
                    <path d="M -22,-14 L -8,-14 L -3,-9 L 20,-9 C 22.2,-9 24,-7.2 24,-5 L 24,14 C 24,16.2 22.2,18 20,18 L -22,18 C -24.2,18 -26,16.2 -26,14 L -26,-10 C -26,-12.2 -24.2,-14 -22,-14 Z" fill="none" stroke="#a855f7" stroke-width="2.5" stroke-linejoin="round"/>
                    <line x1="-26" y1="-5" x2="24" y2="-5" stroke="#a855f7" stroke-width="1.8" opacity="0.6"/>
                    <path d="M -9,3 L 7,3 M 3,-1 L 7,3 L 3,7" fill="none" stroke="#e9d5ff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M 7,10 L -9,10 M -5,6 L -9,10 L -5,14" fill="none" stroke="#e9d5ff" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
                </g>
            """
        }
    }

    template_with_label = """<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">
  <rect width="144" height="144" fill="#10121b"/>
  <rect x="20" y="16" width="104" height="76" rx="18" ry="18" fill="#141724" stroke="{stroke}" stroke-width="3"/>
  {glyph}
  <text x="72" y="122" font-family="'DejaVu Sans', 'Ubuntu', 'Inter', sans-serif" font-weight="bold" font-size="16" fill="#ffffff" text-anchor="middle" letter-spacing="0.75">{label}</text>
</svg>"""

    for name, data in icons.items():
        svg_code = template_with_label.format(
            stroke=data["stroke"],
            glyph=data["glyph"],
            label=data["label"]
        )
        svg_file = WORKSPACE_FRONTEND / f"{name}.svg"
        svg_file.write_text(svg_code, encoding="utf-8")
        png_target1 = ICONS_DIR / f"{name}.png"
        png_target2 = WORKSPACE_FRONTEND / f"{name}.png"
        subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target1)], check=True)
        subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target2)], check=True)

    # Nawigacja bez tekstu: Strzałki < i > oraz powrót ←
    arrows = {
        "icon_arrow_left": """<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">
  <rect width="144" height="144" fill="#10121b"/>
  <rect x="20" y="20" width="104" height="104" rx="22" ry="22" fill="#141724" stroke="#475569" stroke-width="2.5"/>
  <g transform="translate(72, 72)">
    <polyline points="6,-20 -14,0 6,20" fill="none" stroke="#e2e8f0" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>
  </g>
</svg>""",
        "icon_arrow_right": """<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">
  <rect width="144" height="144" fill="#10121b"/>
  <rect x="20" y="20" width="104" height="104" rx="22" ry="22" fill="#141724" stroke="#475569" stroke-width="2.5"/>
  <g transform="translate(72, 72)">
    <polyline points="-6,-20 14,0 -6,20" fill="none" stroke="#e2e8f0" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>
  </g>
</svg>""",
        "icon_arrow_back": """<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">
  <rect width="144" height="144" fill="#10121b"/>
  <rect x="20" y="20" width="104" height="104" rx="22" ry="22" fill="#141724" stroke="#6366f1" stroke-width="2.5"/>
  <g transform="translate(72, 72)">
    <path d="M 16,0 L -14,0 M -4,-12 L -16,0 L -4,12" fill="none" stroke="#c7d2fe" stroke-width="4.5" stroke-linecap="round" stroke-linejoin="round"/>
  </g>
</svg>"""
    }

    for name, s_code in arrows.items():
        svg_file = WORKSPACE_FRONTEND / f"{name}.svg"
        svg_file.write_text(s_code, encoding="utf-8")
        png_target1 = ICONS_DIR / f"{name}.png"
        png_target2 = WORKSPACE_FRONTEND / f"{name}.png"
        subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target1)], check=True)
        subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target2)], check=True)

def generate_task_button_icon(problem_id: str, title: str) -> Path:
    """Generuje ikonę w submenu: SAM TEKST, bez grafik, wypełniający maksymalny obszar klawisza."""
    import textwrap
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    clean_title = (title or problem_id).upper().strip()
    words = clean_title.split()

    if len(clean_title) <= 5:
        lines = [clean_title]
        fs = 30
    elif len(clean_title) <= 8:
        lines = [clean_title]
        fs = 23
    elif len(clean_title) <= 10 and len(words) == 1:
        lines = [clean_title]
        fs = 19
    elif len(words) >= 2:
        lines = textwrap.wrap(clean_title, width=12)
        if len(lines) == 2:
            max_l = max(len(l) for l in lines)
            fs = 16 if max_l <= 11 else 13.5
        elif len(lines) >= 3:
            lines = textwrap.wrap(clean_title, width=13)[:3]
            fs = 13 if max(len(l) for l in lines) <= 12 else 11.5
    else:
        fs = max(8.0, min(18.0, 120.0 / (len(clean_title) * 0.65)))
        lines = [clean_title]

    if len(lines) == 1:
        text_tags = f'<text x="72" y="80" font-family="\'DejaVu Sans\', \'Ubuntu\', \'Inter\', sans-serif" font-weight="900" font-size="{fs}" fill="#ffffff" text-anchor="middle">{lines[0]}</text>'
    elif len(lines) == 2:
        line_height = fs * 1.35
        y1 = 72 - (line_height / 2) + (fs * 0.35)
        y2 = y1 + line_height
        text_tags = f'''<text x="72" y="{y1:.1f}" font-family="\'DejaVu Sans\', \'Ubuntu\', \'Inter\', sans-serif" font-weight="900" font-size="{fs}" fill="#ffffff" text-anchor="middle">{lines[0]}</text>
  <text x="72" y="{y2:.1f}" font-family="\'DejaVu Sans\', \'Ubuntu\', \'Inter\', sans-serif" font-weight="900" font-size="{fs}" fill="#ffffff" text-anchor="middle">{lines[1]}</text>'''
    else:
        line_height = fs * 1.3
        y1 = 72 - line_height + (fs * 0.35)
        y2 = 72 + (fs * 0.35)
        y3 = y2 + line_height
        text_tags = f'''<text x="72" y="{y1:.1f}" font-family="\'DejaVu Sans\', \'Ubuntu\', \'Inter\', sans-serif" font-weight="900" font-size="{fs}" fill="#ffffff" text-anchor="middle">{lines[0]}</text>
  <text x="72" y="{y2:.1f}" font-family="\'DejaVu Sans\', \'Ubuntu\', \'Inter\', sans-serif" font-weight="900" font-size="{fs}" fill="#ffffff" text-anchor="middle">{lines[1]}</text>
  <text x="72" y="{y3:.1f}" font-family="\'DejaVu Sans\', \'Ubuntu\', \'Inter\', sans-serif" font-weight="900" font-size="{fs}" fill="#ffffff" text-anchor="middle">{lines[2]}</text>'''

    svg_code = f"""<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">
  <rect width="144" height="144" fill="#10121b"/>
  <rect x="6" y="6" width="132" height="132" rx="20" ry="20" fill="#151829" stroke="#06b6d4" stroke-width="2.5"/>
  {text_tags}
</svg>"""

    svg_file = WORKSPACE_FRONTEND / f"task_{problem_id.lower()}.svg"
    svg_file.write_text(svg_code, encoding="utf-8")
    png_target1 = ICONS_DIR / f"task_{problem_id.lower()}.png"
    png_target2 = WORKSPACE_FRONTEND / f"task_{problem_id.lower()}.png"
    subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target1)], check=True)
    subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target2)], check=True)
    return png_target1


def generate_active_task_badge(problem_id: str, title: str) -> Path:
    """Generuje badge aktywnego zadania na górną linię Stream Decka (2x0)."""
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    clean_title = (title or problem_id).upper().strip()
    words = clean_title.split()
    short_code = problem_id.upper()

    if len(clean_title) <= 8:
        display_title = clean_title
        fs = 20
    elif len(words) >= 2 and len(words[0]) <= 8:
        display_title = words[0]
        fs = 18
    else:
        display_title = short_code
        fs = 22

    svg = f"""<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">
  <rect width="144" height="144" fill="#10121b"/>
  <rect x="8" y="8" width="128" height="128" rx="22" ry="22" fill="#141727" stroke="#06b6d4" stroke-width="2.5"/>
  <rect x="34" y="20" width="76" height="22" rx="11" ry="11" fill="#083344"/>
  <text x="72" y="35" font-family="'DejaVu Sans', 'Ubuntu', 'Inter', sans-serif" font-weight="bold" font-size="9" fill="#38bdf8" text-anchor="middle" letter-spacing="0.8">● ZADANIE</text>
  <text x="72" y="80" font-family="'DejaVu Sans', 'Ubuntu', 'Inter', sans-serif" font-weight="900" font-size="{fs}" fill="#ffffff" text-anchor="middle">{display_title}</text>
  <text x="72" y="108" font-family="'DejaVu Sans', 'Ubuntu', 'Inter', sans-serif" font-weight="bold" font-size="11" fill="#94a3b8" text-anchor="middle">[{short_code}]</text>
</svg>"""

    svg_file = WORKSPACE_FRONTEND / f"badge_{problem_id.lower()}.svg"
    svg_file.write_text(svg, encoding="utf-8")
    png_target1 = ICONS_DIR / f"badge_{problem_id.lower()}.png"
    png_target2 = WORKSPACE_FRONTEND / f"badge_{problem_id.lower()}.png"
    subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target1)], check=True)
    subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target2)], check=True)
    return png_target1


if __name__ == "__main__":
    generate_icons()
