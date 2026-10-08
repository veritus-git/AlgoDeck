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
                <!-- Rocket glyph centered at (72, 54) -->
                <g transform="translate(72, 54)">
                    <!-- Rocket body -->
                    <path d="M 0,-24 C 8,-16 12,-4 12,12 L -12,12 C -12,-4 -8,-16 0,-24 Z" fill="#10b981"/>
                    <!-- Rocket window -->
                    <circle cx="0" cy="-6" r="4.5" fill="#10121b"/>
                    <circle cx="0" cy="-6" r="2.5" fill="#34d399"/>
                    <!-- Left fin -->
                    <path d="M -12,4 L -20,16 L -12,14 Z" fill="#059669"/>
                    <!-- Right fin -->
                    <path d="M 12,4 L 20,16 L 12,14 Z" fill="#059669"/>
                    <!-- Thruster flame -->
                    <path d="M -6,14 Q 0,26 0,26 Q 0,26 6,14 Z" fill="#f59e0b"/>
                    <path d="M -3,14 Q 0,21 0,21 Q 0,21 3,14 Z" fill="#fef08a"/>
                </g>
            """
        },
        "icon_play_sec": {
            "label": "ODPAL",
            "stroke": "#06b6d4", # Cyan
            "glyph": """
                <!-- Play triangle centered at (72, 54) -->
                <g transform="translate(72, 54)">
                    <path d="M -12,-18 C -12,-20 -9.5,-21.5 -7.5,-20.2 L 18,-4 C 19.8,-2.8 19.8,0.2 18,1.4 L -7.5,17.6 C -9.5,18.9 -12,17.4 -12,15.4 Z" fill="#06b6d4"/>
                </g>
            """
        },
        "icon_kill_sec": {
            "label": "KILL",
            "stroke": "#f43f5e", # Rose / Red
            "glyph": """
                <!-- Lightning / Zap bolt centered at (72, 54) -->
                <g transform="translate(72, 54)">
                    <path d="M 2,-22 L -16,-2 L -3,-2 L -6,22 L 16,2 L 3,2 Z" fill="#f43f5e"/>
                </g>
            """
        },
        "icon_vscode_sec": {
            "label": "VS CODE",
            "stroke": "#0284c7", # VS Code Blue
            "glyph": """
                <!-- VS Code Ribbon centered at (72, 54) -->
                <g transform="translate(72, 54) scale(1.15)">
                    <path d="M 13.5,-16.5 C 14.5,-17.2 16,-16.5 16,-15.2 L 16,15.2 C 16,16.5 14.5,17.2 13.5,16.5 L 3,8.5 L -10.5,18 C -11.5,18.8 -13,18.2 -13,17 L -13,15.5 L -1.5,5 L -15.2,-4.5 C -16,-5.2 -15.8,-6.5 -14.8,-7 L -10,-9.5 C -9.2,-10 -8.2,-9.8 -7.5,-9.2 L 3,-1.5 Z" fill="#0284c7"/>
                    <path d="M 3,-1.5 L 13.5,-9.5 L 16,-8 L 3,1.5 Z" fill="#38bdf8" opacity="0.85"/>
                    <path d="M 3,-1.5 L -7.5,-9.2 L -14.8,-7 L 3,5 Z" fill="#007acc"/>
                    <path d="M 16,15.2 L 3,5 L 16,-4 Z" fill="#0369a1"/>
                </g>
            """
        },
        "icon_task_sec": {
            "label": "ZADANIE",
            "stroke": "#a855f7", # Purple
            "glyph": """
                <!-- Project Folder / Switch centered at (72, 54) -->
                <g transform="translate(72, 54)">
                    <!-- Folder body -->
                    <path d="M -18,-14 L -6,-14 L -2,-10 L 18,-10 C 20,-10 21,-9 21,-7 L 21,12 C 21,14 20,15 18,15 L -18,15 C -20,15 -21,14 -21,12 L -21,-12 C -21,-14 -20,-14 -18,-14 Z" fill="none" stroke="#a855f7" stroke-width="2.5"/>
                    <!-- Switch / Refresh arrows in folder -->
                    <path d="M -6,1 A 6 6 0 0 1 6,1" fill="none" stroke="#c084fc" stroke-width="2.2" stroke-linecap="round"/>
                    <polyline points="4,-1 7,1 5,4" fill="none" stroke="#c084fc" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M 6,5 A 6 6 0 0 1 -6,5" fill="none" stroke="#c084fc" stroke-width="2.2" stroke-linecap="round"/>
                    <polyline points="-4,7 -7,5 -5,2" fill="none" stroke="#c084fc" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </g>
            """
        },
        "icon_next_sec": {
            "label": "NAST",
            "stroke": "#818cf8", # Indigo
            "glyph": """
                <!-- Next arrows >>| centered at (72, 54) -->
                <g transform="translate(72, 54)">
                    <path d="M -18,-14 L -4,0 L -18,14 Z" fill="#818cf8"/>
                    <path d="M -4,-14 L 10,0 L -4,14 Z" fill="#818cf8"/>
                    <rect x="13" y="-14" width="4.5" height="28" rx="2" fill="#818cf8"/>
                </g>
            """
        },
        "icon_prev_sec": {
            "label": "POPRZ",
            "stroke": "#818cf8", # Indigo
            "glyph": """
                <!-- Prev arrows |<< centered at (72, 54) -->
                <g transform="translate(72, 54)">
                    <rect x="-17.5" y="-14" width="4.5" height="28" rx="2" fill="#818cf8"/>
                    <path d="M 4,-14 L -10,0 L 4,14 Z" fill="#818cf8"/>
                    <path d="M 18,-14 L 4,0 L 18,14 Z" fill="#818cf8"/>
                </g>
            """
        }
    }

    template = """<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144">
  <rect width="144" height="144" fill="#10121b"/>
  <!-- Rounded Frame Badge -->
  <rect x="20" y="16" width="104" height="76" rx="18" ry="18" fill="#141724" stroke="{stroke}" stroke-width="3"/>
  <!-- Icon Glyph -->
  {glyph}
  <!-- Bottom Text Label -->
  <text x="72" y="122" font-family="'DejaVu Sans', 'Ubuntu', 'Inter', sans-serif" font-weight="bold" font-size="16" fill="#ffffff" text-anchor="middle" letter-spacing="0.75">{label}</text>
</svg>"""

    for name, data in icons.items():
        svg_code = template.format(
            stroke=data["stroke"],
            glyph=data["glyph"],
            label=data["label"]
        )
        svg_file = WORKSPACE_FRONTEND / f"{name}.svg"
        svg_file.write_text(svg_code, encoding="utf-8")

        png_target1 = ICONS_DIR / f"{name}.png"
        png_target2 = WORKSPACE_FRONTEND / f"{name}.png"

        # Convert SVG to PNG 144x144 using ImageMagick
        subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target1)], check=True)
        subprocess.run(["convert", "-background", "none", str(svg_file), "-resize", "144x144", str(png_target2)], check=True)
        print(f"Generated: {name}.png")

if __name__ == "__main__":
    generate_icons()
