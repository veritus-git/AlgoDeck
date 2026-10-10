import os
import shutil
import textwrap
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ICONS_DIR = Path.home() / ".var/app/com.core447.StreamController/data/custom_icons"
WORKSPACE_FRONTEND = Path(__file__).resolve().parent.parent.parent / "frontend" / "icons"

def _get_font(size: int) -> ImageFont.ImageFont:
    """Wyszukuje systemowy font TrueType bez potrzeby dodatkowych instalacji."""
    candidates = [
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/google-noto/NotoSans-Bold.ttf"
    ]
    for c in candidates:
        if os.path.exists(c):
            try:
                return ImageFont.truetype(c, size)
            except Exception:
                pass
    return ImageFont.load_default()

def _create_canvas_black():
    """Tworzy bazowe, całkowicie czarne tło OLED (144x144) bez żadnych obramowań ani kart."""
    img = Image.new("RGBA", (144, 144), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)
    return img, draw

def generate_letter_icon(char: str) -> Path:
    """
    Pojedyncza duża, geometryczna litera (np. 'C', 'H', 'W')
    na całkowicie czarnym tle (#000000) dla górnego rzędu Stream Decka.
    """
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    img, draw = _create_canvas_black()
    clean_char = (char or "?").upper()[:1]
    font = _get_font(84)
    # Czysta, wyrazista litera w neonowym cyjanie
    draw.text((72, 68), clean_char, fill=(56, 189, 248, 255), font=font, anchor="mm")

    filename = f"letter_{clean_char.lower()}.png"
    p1 = ICONS_DIR / filename
    p2 = WORKSPACE_FRONTEND / filename
    img.save(p1, format="PNG")
    img.save(p2, format="PNG")
    return p1

def generate_vscode_icon() -> Image.Image:
    """Prosty, geometryczny symbol kodu: < / > na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_black()
    c_cyan = (56, 189, 248, 255)
    c_slash = (148, 163, 184, 255)
    
    # Lewy nawias <
    draw.line([(42, 72), (60, 48)], fill=c_cyan, width=8)
    draw.line([(42, 72), (60, 96)], fill=c_cyan, width=8)
    # Ukośnik /
    draw.line([(68, 100), (76, 44)], fill=c_slash, width=7)
    # Prawy nawias >
    draw.line([(102, 72), (84, 48)], fill=c_cyan, width=8)
    draw.line([(102, 72), (84, 96)], fill=c_cyan, width=8)
    return img

def generate_test_icon() -> Image.Image:
    """
    Ujednolicony geometryczny symbol testów: wyrazisty checkmark ✓
    w szmaragdowej zieleni, idealnie pasujący do trójkąta ▶, kwadratu ⏹ i siatki ⊞.
    """
    img, draw = _create_canvas_black()
    c_emerald = (16, 185, 129, 255)
    # Czysty, geometryczny checkmark o stałej grubości
    draw.line([(34, 74), (60, 100)], fill=c_emerald, width=14)
    draw.line([(60, 100), (110, 44)], fill=c_emerald, width=14)
    draw.ellipse([27, 67, 41, 81], fill=c_emerald)
    draw.ellipse([53, 93, 67, 107], fill=c_emerald)
    draw.ellipse([103, 37, 117, 51], fill=c_emerald)
    return img

def generate_play_icon() -> Image.Image:
    """Czysty, geometryczny zielony trójkąt Play ▶ na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_black()
    c_green = (34, 197, 94, 255)
    pts = [(48, 38), (108, 72), (48, 106)]
    draw.polygon(pts, fill=c_green)
    return img

def generate_kill_icon() -> Image.Image:
    """Czysty, geometryczny czerwony kwadrat Stop ⏹ na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_black()
    c_red = (239, 68, 68, 255)
    draw.rounded_rectangle((40, 40, 104, 104), radius=8, fill=c_red)
    return img

def generate_menu_icon() -> Image.Image:
    """Czyste, geometryczne 4 kwadraciki ⊞ (Zadania/Galeria) na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_black()
    c_indigo = (129, 140, 248, 255)
    r = 5
    draw.rounded_rectangle((38, 38, 68, 68), radius=r, fill=c_indigo)
    draw.rounded_rectangle((76, 38, 106, 68), radius=r, fill=c_indigo)
    draw.rounded_rectangle((38, 76, 68, 106), radius=r, fill=c_indigo)
    draw.rounded_rectangle((76, 76, 106, 106), radius=r, fill=c_indigo)
    return img

def generate_arrow_left_icon() -> Image.Image:
    """Minimalistyczny szewron w lewo ‹ na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_black()
    c_slate = (203, 213, 225, 255)
    draw.line([(88, 44), (56, 72)], fill=c_slate, width=9)
    draw.line([(56, 72), (88, 100)], fill=c_slate, width=9)
    draw.ellipse([51, 67, 61, 77], fill=c_slate)
    return img

def generate_arrow_right_icon() -> Image.Image:
    """Minimalistyczny szewron w prawo › na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_black()
    c_slate = (203, 213, 225, 255)
    draw.line([(56, 44), (88, 72)], fill=c_slate, width=9)
    draw.line([(88, 72), (56, 100)], fill=c_slate, width=9)
    draw.ellipse([83, 67, 93, 77], fill=c_slate)
    return img

def generate_arrow_back_icon() -> Image.Image:
    """Minimalistyczna geometryczna strzałka powrotu ← na całkowicie czarnym tle (#000000)."""
    img, draw = _create_canvas_black()
    c_slate = (203, 213, 225, 255)
    draw.line([(42, 72), (104, 72)], fill=c_slate, width=9)
    draw.line([(42, 72), (68, 46)], fill=c_slate, width=9)
    draw.line([(42, 72), (68, 98)], fill=c_slate, width=9)
    draw.ellipse([37, 67, 47, 77], fill=c_slate)
    draw.ellipse([99, 67, 109, 77], fill=c_slate)
    draw.ellipse([63, 41, 73, 51], fill=c_slate)
    draw.ellipse([63, 93, 73, 103], fill=c_slate)
    return img

# ----------------- KAFLE ZADAŃ W GALERII (ALGO_MENU) -----------------
def generate_task_button_icon(problem_id: str, title: str, is_active: bool = False) -> Path:
    """
    Kafel zadania do widoku zadań (ALGO_MENU) - pozostawiony dokładnie 1:1 tak jak jest,
    zgodnie z życzeniem użytkownika (pełna nazwa, elegancka ramka i wskaźnik).
    """
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    clean_title = (title or problem_id).upper().strip()
    words = clean_title.split()

    img = Image.new("RGBA", (144, 144), (16, 18, 27, 255))
    draw = ImageDraw.Draw(img)
    
    outline_color = (56, 189, 248, 255) if is_active else (6, 182, 212, 255)
    draw.rounded_rectangle((6, 6, 138, 138), radius=20, fill=(21, 24, 41, 255), outline=outline_color, width=3)

    if len(clean_title) <= 5:
        lines = [clean_title]
        fs = 26
    elif len(clean_title) <= 8:
        lines = [clean_title]
        fs = 21
    elif len(clean_title) <= 10 and len(words) == 1:
        lines = [clean_title]
        fs = 18
    elif len(words) >= 2:
        lines = textwrap.wrap(clean_title, width=11)[:3]
        fs = 15 if len(lines) <= 2 else 12.5
    else:
        lines = [clean_title[:10]]
        fs = 16

    font = _get_font(int(fs))
    if len(lines) == 1:
        draw.text((72, 72), lines[0], fill=(255, 255, 255, 255), font=font, anchor="mm")
    elif len(lines) == 2:
        draw.text((72, 54), lines[0], fill=(255, 255, 255, 255), font=font, anchor="mm")
        draw.text((72, 90), lines[1], fill=(255, 255, 255, 255), font=font, anchor="mm")
    else:
        draw.text((72, 42), lines[0], fill=(255, 255, 255, 255), font=font, anchor="mm")
        draw.text((72, 72), lines[1], fill=(255, 255, 255, 255), font=font, anchor="mm")
        draw.text((72, 102), lines[2], fill=(255, 255, 255, 255), font=font, anchor="mm")

    png_target1 = ICONS_DIR / f"task_{problem_id.lower()}.png"
    png_target2 = WORKSPACE_FRONTEND / f"task_{problem_id.lower()}.png"
    img.save(png_target1, format="PNG")
    img.save(png_target2, format="PNG")
    return png_target1

def generate_all_base_icons():
    """Generuje komplet czystych geometrycznych ikon akcji na czarnym tle."""
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    icons_map = {
        "icon_vscode_sec.png": generate_vscode_icon(),
        "icon_test_sec.png": generate_test_icon(),
        "icon_play_sec.png": generate_play_icon(),
        "icon_kill_sec.png": generate_kill_icon(),
        "icon_task_sec.png": generate_menu_icon(),
        "icon_arrow_left.png": generate_arrow_left_icon(),
        "icon_arrow_right.png": generate_arrow_right_icon(),
        "icon_arrow_back.png": generate_arrow_back_icon(),
    }

    for name, img in icons_map.items():
        dst1 = ICONS_DIR / name
        dst2 = WORKSPACE_FRONTEND / name
        img.save(dst1, format="PNG")
        img.save(dst2, format="PNG")

if __name__ == "__main__":
    generate_all_base_icons()
    print("Wygenerowano geometryczne ikony na czarnym tle.")
