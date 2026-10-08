import os
import shutil
import textwrap
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ICONS_DIR = Path.home() / ".var/app/com.core447.StreamController/data/custom_icons"
WORKSPACE_FRONTEND = Path(__file__).resolve().parent.parent.parent / "frontend" / "icons"

def _get_font(size: int) -> ImageFont.ImageFont:
    """Wyszukuje zainstalowany font systemowy bez konieczności instalowania dodatkowych pakietów."""
    candidates = [
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf"
    ]
    for c in candidates:
        if os.path.exists(c):
            try:
                return ImageFont.truetype(c, size)
            except Exception:
                pass
    return ImageFont.load_default()

def generate_active_task_badge(problem_id: str, title: str) -> Path:
    """
    Dynamicznie generuje badge aktywnego zadania na górną linię Stream Decka (klawisz 2x0).
    Działa w 100% w czystym Pythonie (Pillow) bez zależności od ImageMagick ani zewnętrznych binarek.
    """
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    clean_title = (title or problem_id).upper().strip()
    words = clean_title.split()
    short_code = problem_id.upper()

    img = Image.new("RGBA", (144, 144), (16, 18, 27, 255))
    draw = ImageDraw.Draw(img)

    # Ramka z ciemnym tłem i cyjanowym obramowaniem
    draw.rounded_rectangle((6, 6, 138, 138), radius=22, fill=(20, 23, 39, 255), outline=(6, 182, 212, 255), width=3)
    # Górna pigułka ● ZADANIE
    draw.rounded_rectangle((34, 16, 110, 38), radius=11, fill=(8, 51, 68, 255))
    draw.text((72, 27), "● ZADANIE", fill=(56, 189, 248, 255), font=_get_font(10), anchor="mm")

    # Formatowanie tytułu w zależności od długości
    if len(clean_title) <= 8:
        lines = [clean_title]
        fs = 20
    elif len(words) >= 2:
        lines = textwrap.wrap(clean_title, width=11)[:2]
        fs = 15
    else:
        lines = [clean_title[:8]]
        fs = 18

    font_main = _get_font(fs)
    if len(lines) == 1:
        draw.text((72, 75), lines[0], fill=(255, 255, 255, 255), font=font_main, anchor="mm")
    else:
        draw.text((72, 65), lines[0], fill=(255, 255, 255, 255), font=font_main, anchor="mm")
        draw.text((72, 85), lines[1], fill=(255, 255, 255, 255), font=font_main, anchor="mm")

    # Dolny kod zadania w nawiasie [ABC]
    draw.text((72, 112), f"[{short_code}]", fill=(148, 163, 184, 255), font=_get_font(12), anchor="mm")

    png_target1 = ICONS_DIR / f"badge_{problem_id.lower()}.png"
    png_target2 = WORKSPACE_FRONTEND / f"badge_{problem_id.lower()}.png"
    img.save(png_target1, format="PNG")
    img.save(png_target2, format="PNG")
    return png_target1

def generate_task_button_icon(problem_id: str, title: str) -> Path:
    """
    Dynamicznie generuje kaflowy przycisk zadania do submenu ALGO_MENU (pełny, czytelny tekst).
    Działa w 100% w czystym Pythonie (Pillow).
    """
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    clean_title = (title or problem_id).upper().strip()
    words = clean_title.split()

    img = Image.new("RGBA", (144, 144), (16, 18, 27, 255))
    draw = ImageDraw.Draw(img)
    draw.rounded_rectangle((6, 6, 138, 138), radius=20, fill=(21, 24, 41, 255), outline=(6, 182, 212, 255), width=3)

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

def generate_icons():
    """Kopiuje bazowe ikony i generuje grafiki dla aktualnie zarejestrowanych zadań."""
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    # Kopiuj gotowe ikony akcji jeśli istnieją
    for icon_name in [
        "icon_vscode_sec.png", "icon_test_sec.png", "icon_play_sec.png",
        "icon_kill_sec.png", "icon_task_sec.png", "icon_arrow_left.png",
        "icon_arrow_right.png", "icon_arrow_back.png"
    ]:
        src = WORKSPACE_FRONTEND / icon_name
        dst = ICONS_DIR / icon_name
        if src.exists() and not dst.exists():
            try:
                shutil.copy2(src, dst)
            except Exception:
                pass

if __name__ == "__main__":
    generate_icons()
