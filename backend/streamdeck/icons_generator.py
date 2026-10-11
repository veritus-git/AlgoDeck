import os
import shutil
import textwrap
from pathlib import Path
try:
    import cairo
except ImportError:
    cairo = None
from PIL import Image, ImageDraw, ImageFont

ICONS_DIR = Path.home() / ".var/app/com.core447.StreamController/data/custom_icons"
WORKSPACE_FRONTEND = Path(__file__).resolve().parent.parent.parent / "frontend" / "icons"

def _ensure_base_icon(filename: str) -> Path:
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    p1 = ICONS_DIR / filename
    p2 = WORKSPACE_FRONTEND / filename
    if not p1.exists() and p2.exists():
        try:
            shutil.copy2(p2, p1)
        except Exception:
            pass
    return p1

def _save_cairo_surf(surf, filename: str) -> Path:
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)
    p1 = ICONS_DIR / filename
    p2 = WORKSPACE_FRONTEND / filename
    if surf is not None:
        try:
            surf.write_to_png(str(p1))
            surf.write_to_png(str(p2))
            return p1
        except Exception:
            pass
    return _ensure_base_icon(filename)

def _new_cairo_surface():
    if cairo is None:
        return None, None
    surf = cairo.ImageSurface(cairo.FORMAT_ARGB32, 144, 144)
    cr = cairo.Context(surf)
    cr.set_source_rgb(0, 0, 0)
    cr.paint()
    return surf, cr

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

SCALE = 4
SIZE_HI = 144 * SCALE
SIZE_FINAL = (144, 144)

def _create_canvas_hi():
    img = Image.new("RGBA", (SIZE_HI, SIZE_HI), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)
    return img, draw

def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(SIZE_FINAL, resample=Image.Resampling.LANCZOS)

def generate_letter_icon(char: str) -> Path:
    """
    Pojedyncza duża, geometryczna litera (np. 'C', 'H', 'W')
    w CZYSTEJ BIELI (255, 255, 255) na całkowicie czarnym tle (#000000).
    """
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    clean_char = (char or "?").upper()[:1]
    filename = f"letter_{clean_char.lower()}.png"
    p1 = ICONS_DIR / filename
    p2 = WORKSPACE_FRONTEND / filename

    if p1.exists():
        return p1

    img, draw = _create_canvas_hi()
    font = _get_font(84 * SCALE)
    draw.text((288, 272), clean_char, fill=(255, 255, 255, 255), font=font, anchor="mm")

    final_img = _downsample(img)
    final_img.save(p1, format="PNG")
    final_img.save(p2, format="PNG")
    return p1

def generate_all_alphabet_letters():
    """Pre-generuje cały polski alfabet (A-Z + Ą, Ć, Ę, itd.) oraz cyfry."""
    chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZĄĆĘŁŃÓŚŹŻ0123456789_-"
    for ch in chars:
        generate_letter_icon(ch)

def generate_equals_icon() -> Path:
    """
    Subtelny akcent '=': lekko przyciemniony szary (#64748b),
    mniejszy rozmiar i ostre krawędzie 90°.
    """
    if cairo is None:
        return _ensure_base_icon("icon_equals.png")
    surf, cr = _new_cairo_surface()
    cr.set_source_rgb(100/255, 116/255, 139/255)  # #64748b slate-500
    cr.rectangle(53, 60, 38, 6)
    cr.rectangle(53, 78, 38, 6)
    cr.fill()
    return _save_cairo_surf(surf, "icon_equals.png")

def generate_plus_icon() -> Path:
    """
    Duży, geometryczny i idealnie wyśrodkowany '+' w jasnoszarym odcieniu (#e2e8f0).
    Wielkość (56x56 px) jest ujednolicona z pozostałymi ikonami środkowego rzędu.
    """
    if cairo is None:
        return _ensure_base_icon("icon_plus.png")
    surf, cr = _new_cairo_surface()
    cr.set_source_rgb(226/255, 232/255, 240/255)  # #e2e8f0 jasny szary
    cr.rectangle(44, 67, 56, 10)
    cr.rectangle(67, 44, 10, 56)
    cr.fill()
    return _save_cairo_surf(surf, "icon_plus.png")

def generate_test_icon() -> Path:
    """
    Wektorowy zielony tick ✓ z ostrym wierzchołkiem (miter join)
    oraz prostokątnymi końcówkami (square cap).
    """
    if cairo is None:
        return _ensure_base_icon("icon_test_sec.png")
    surf, cr = _new_cairo_surface()
    cr.set_line_width(13.0)
    cr.set_line_cap(cairo.LINE_CAP_SQUARE)
    cr.set_line_join(cairo.LINE_JOIN_MITER)
    cr.set_miter_limit(10.0)
    cr.set_source_rgb(16/255, 185/255, 129/255)  # Emerald #10b981
    cr.move_to(40, 72)
    cr.line_to(62, 94)
    cr.line_to(104, 52)
    cr.stroke()
    return _save_cairo_surf(surf, "icon_test_sec.png")

def generate_play_icon() -> Path:
    """Czysty geometryczny zielony trójkąt Play ▶ (54x54 px) z ostrymi rogami."""
    if cairo is None:
        return _ensure_base_icon("icon_play_sec.png")
    surf, cr = _new_cairo_surface()
    cr.set_source_rgb(34/255, 197/255, 94/255)  # Green #22c55e
    cr.set_line_join(cairo.LINE_JOIN_MITER)
    cr.set_miter_limit(10.0)
    cr.move_to(48, 45)
    cr.line_to(102, 72)
    cr.line_to(48, 99)
    cr.close_path()
    cr.fill()
    return _save_cairo_surf(surf, "icon_play_sec.png")

def generate_kill_icon() -> Path:
    """Czysty geometryczny czerwony kwadrat Stop ⏹ (54x54 px) z ostrymi kątami."""
    if cairo is None:
        return _ensure_base_icon("icon_kill_sec.png")
    surf, cr = _new_cairo_surface()
    cr.set_source_rgb(239/255, 68/255, 68/255)  # Red #ef4444
    cr.rectangle(45, 45, 54, 54)
    cr.fill()
    return _save_cairo_surf(surf, "icon_kill_sec.png")

def generate_menu_icon() -> Path:
    """Czyste, geometryczne 4 kwadraty ⊞ (54x54 px) z ostrymi rogami."""
    if cairo is None:
        return _ensure_base_icon("icon_task_sec.png")
    surf, cr = _new_cairo_surface()
    cr.set_source_rgb(129/255, 140/255, 248/255)  # Indigo #818cf8
    cr.rectangle(45, 45, 23, 23)
    cr.rectangle(76, 45, 23, 23)
    cr.rectangle(45, 76, 23, 23)
    cr.rectangle(76, 76, 23, 23)
    cr.fill()
    return _save_cairo_surf(surf, "icon_task_sec.png")

def generate_arrow_left_icon(disabled: bool = False) -> Path:
    """Kompaktowy szewron ‹ (~30 px) dopasowany do skali znaku '='."""
    filename = "icon_arrow_left_disabled.png" if disabled else "icon_arrow_left.png"
    if cairo is None:
        return _ensure_base_icon(filename)
    surf, cr = _new_cairo_surface()
    cr.set_line_width(7.0)
    cr.set_line_cap(cairo.LINE_CAP_SQUARE)
    cr.set_line_join(cairo.LINE_JOIN_MITER)
    cr.set_miter_limit(10.0)
    if disabled:
        cr.set_source_rgb(51/255, 65/255, 85/255)  # #334155
    else:
        cr.set_source_rgb(1.0, 1.0, 1.0)  # #ffffff
    cr.move_to(79, 57)
    cr.line_to(64, 72)
    cr.line_to(79, 87)
    cr.stroke()
    return _save_cairo_surf(surf, filename)

def generate_arrow_right_icon(disabled: bool = False) -> Path:
    """Kompaktowy szewron › (~30 px) dopasowany do skali znaku '='."""
    filename = "icon_arrow_right_disabled.png" if disabled else "icon_arrow_right.png"
    if cairo is None:
        return _ensure_base_icon(filename)
    surf, cr = _new_cairo_surface()
    cr.set_line_width(7.0)
    cr.set_line_cap(cairo.LINE_CAP_SQUARE)
    cr.set_line_join(cairo.LINE_JOIN_MITER)
    cr.set_miter_limit(10.0)
    if disabled:
        cr.set_source_rgb(51/255, 65/255, 85/255)  # #334155
    else:
        cr.set_source_rgb(1.0, 1.0, 1.0)  # #ffffff
    cr.move_to(65, 57)
    cr.line_to(80, 72)
    cr.line_to(65, 87)
    cr.stroke()
    return _save_cairo_surf(surf, filename)

def generate_arrow_back_icon() -> Path:
    """Geometryczna strzałka ← z ostrym grotem miter dla widoku menu."""
    if cairo is None:
        return _ensure_base_icon("icon_arrow_back.png")
    surf, cr = _new_cairo_surface()
    cr.set_line_width(11.0)
    cr.set_line_cap(cairo.LINE_CAP_SQUARE)
    cr.set_line_join(cairo.LINE_JOIN_MITER)
    cr.set_miter_limit(10.0)
    cr.set_source_rgb(203/255, 213/255, 225/255)  # Slate-300 #cbd5e1
    cr.move_to(68, 46)
    cr.line_to(42, 72)
    cr.line_to(68, 98)
    cr.stroke()
    cr.move_to(44, 72)
    cr.line_to(102, 72)
    cr.stroke()
    return _save_cairo_surf(surf, "icon_arrow_back.png")

def generate_settings_icon() -> Path:
    """
    Dyskretny, pomniejszony trybik / ustawienia ⚙ na czarnym tle.
    Kolor stonowany szary (#475569 - slate-600), nie rzuca się w oczy.
    """
    if cairo is None:
        return _ensure_base_icon("icon_settings.png")
    import math
    surf, cr = _new_cairo_surface()
    cx, cy = 72, 72
    cr.set_source_rgb(71/255, 85/255, 105/255)  # Slate-600 #475569

    # 6 mniejszych zębów
    cr.set_line_width(5.5)
    cr.set_line_cap(cairo.LINE_CAP_SQUARE)
    for i in range(6):
        a = i * (2 * math.pi / 6)
        cr.move_to(cx + 12 * math.cos(a), cy + 12 * math.sin(a))
        cr.line_to(cx + 22 * math.cos(a), cy + 22 * math.sin(a))
        cr.stroke()

    # Korpus pierścienia
    cr.set_line_width(7.0)
    cr.arc(cx, cy, 13, 0, 2 * math.pi)
    cr.stroke()

    # Wewnętrzny otwór
    cr.set_source_rgb(0, 0, 0)
    cr.arc(cx, cy, 6, 0, 2 * math.pi)
    cr.fill()

    return _save_cairo_surf(surf, "icon_settings.png")

def generate_idle_logo_icons():
    """
    Generuje 3-przyciskowe logo ALGODECK na ekran zachęty (ALGO_IDLE):
    - [ ALGO ] (lewy kafel)
    - [  ⚡  ] (środkowy kafel z geometrycznym piorunem)
    - [ DECK ] (prawy kafel)
    """
    if cairo is None:
        _ensure_base_icon("idle_logo_algo.png")
        _ensure_base_icon("idle_logo_bolt.png")
        _ensure_base_icon("idle_logo_deck.png")
        return

    # 1. ALGO
    surf1, cr1 = _new_cairo_surface()
    cr1.select_font_face("DejaVu Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
    cr1.set_font_size(32.0)
    cr1.set_source_rgb(56/255, 189/255, 248/255)  # Cyan-400 #38bdf8
    ext1 = cr1.text_extents("ALGO")
    cr1.move_to(72 - ext1.width / 2 - ext1.x_bearing, 72 - ext1.height / 2 - ext1.y_bearing)
    cr1.show_text("ALGO")
    _save_cairo_surf(surf1, "idle_logo_algo.png")

    # 2. Piorun ⚡ (geometryczny wektor)
    surf2, cr2 = _new_cairo_surface()
    cr2.set_source_rgb(56/255, 189/255, 248/255)
    cr2.move_to(76, 28)
    cr2.line_to(48, 72)
    cr2.line_to(70, 72)
    cr2.line_to(64, 116)
    cr2.line_to(96, 64)
    cr2.line_to(74, 64)
    cr2.close_path()
    cr2.fill()
    _save_cairo_surf(surf2, "idle_logo_bolt.png")

    # 3. DECK
    surf3, cr3 = _new_cairo_surface()
    cr3.select_font_face("DejaVu Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
    cr3.set_font_size(32.0)
    cr3.set_source_rgb(56/255, 189/255, 248/255)
    ext3 = cr3.text_extents("DECK")
    cr3.move_to(72 - ext3.width / 2 - ext3.x_bearing, 72 - ext3.height / 2 - ext3.y_bearing)
    cr3.show_text("DECK")
    _save_cairo_surf(surf3, "idle_logo_deck.png")

def generate_idle_start_icon() -> Path:
    """
    Przycisk 'ZACZNIJ' w stylu menu głównego gry:
    Szmaragdowy akcent (#10b981), ikona ▶ oraz czytelny napis ZACZNIJ.
    """
    img = Image.new("RGBA", (144, 144), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)

    # Zaokrąglony kafel
    draw.rounded_rectangle((10, 16, 134, 128), radius=16, fill=(6, 78, 59, 255), outline=(16, 185, 129, 255), width=3)

    # Trójkąt ▶
    draw.polygon([(62, 42), (62, 70), (88, 56)], fill=(16, 185, 129, 255))

    # Napis ZACZNIJ
    font = _get_font(20)
    draw.text((72, 98), "ZACZNIJ", fill=(255, 255, 255, 255), font=font, anchor="mm")

    p1 = ICONS_DIR / "idle_btn_start.png"
    p2 = WORKSPACE_FRONTEND / "idle_btn_start.png"
    img.save(p1, format="PNG")
    img.save(p2, format="PNG")
    return p1

def generate_task_number_icon(index: int, total: int) -> Path:
    """Generuje minimalistyczny numer zadania w formacie '1/3'."""
    filename = f"task_num_{index}_{total}.png"
    p1 = ICONS_DIR / filename
    p2 = WORKSPACE_FRONTEND / filename
    if p1.exists():
        return p1

    img, draw = _create_canvas_hi()
    text = f"{index}/{total}" if total > 0 else f"{index}"
    font = _get_font(38 * SCALE)
    draw.text((288, 280), text, fill=(226, 232, 240, 255), font=font, anchor="mm")
    final_img = _downsample(img)
    final_img.save(p1, format="PNG")
    final_img.save(p2, format="PNG")
    return p1

def generate_vscode_icon() -> Path:
    """Ikona kodu zachowana dla wstecznej kompatybilności."""
    if cairo is None:
        return _ensure_base_icon("icon_vscode_sec.png")
    surf, cr = _new_cairo_surface()
    cr.set_line_width(9.0)
    cr.set_line_cap(cairo.LINE_CAP_SQUARE)
    cr.set_line_join(cairo.LINE_JOIN_MITER)
    cr.set_miter_limit(10.0)
    cr.set_source_rgb(56/255, 189/255, 248/255)
    cr.move_to(56, 52); cr.line_to(38, 72); cr.line_to(56, 92); cr.stroke()
    cr.move_to(88, 52); cr.line_to(106, 72); cr.line_to(88, 92); cr.stroke()
    cr.set_source_rgb(148/255, 163/255, 184/255)
    cr.move_to(64, 96); cr.line_to(80, 48); cr.stroke()
    return _save_cairo_surf(surf, "icon_vscode_sec.png")

# ----------------- KAFLE ZADAŃ W GALERII (ALGO_MENU) -----------------
def generate_task_button_icon(problem_id: str, title: str, is_active: bool = False) -> Path:
    """
    Kafel zadania do widoku zadań (ALGO_MENU) - generowany dynamicznie tylko w ICONS_DIR,
    bez zaśmiecania repozytorium gita.
    """
    ICONS_DIR.mkdir(parents=True, exist_ok=True)

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

    png_target = ICONS_DIR / f"task_{problem_id.lower()}.png"
    img.save(png_target, format="PNG")
    return png_target

def generate_all_base_icons():
    """Generuje lub kopiuje komplet czystych ostrych wektorowych ikon akcji oraz alfabet."""
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    # 1. Kopiuj wszystkie pre-renderowane ikony z folderu projektu (błyskawiczne i bez cairo)
    if WORKSPACE_FRONTEND.exists():
        for f in WORKSPACE_FRONTEND.glob("*.png"):
            target = ICONS_DIR / f.name
            if not target.exists():
                try:
                    shutil.copy2(f, target)
                except Exception:
                    pass

    # 2. Jeśli cairo jest zainstalowane, można odświeżyć wektory
    if cairo is not None:
        try:
            generate_test_icon()
            generate_play_icon()
            generate_kill_icon()
            generate_menu_icon()
            generate_equals_icon()
            generate_plus_icon()
            generate_settings_icon()
            generate_idle_logo_icons()
            generate_arrow_left_icon(disabled=False)
            generate_arrow_left_icon(disabled=True)
            generate_arrow_right_icon(disabled=False)
            generate_arrow_right_icon(disabled=True)
            generate_arrow_back_icon()
            generate_vscode_icon()
        except Exception:
            pass

    # 3. Pillow generuje ekran zachęty i litery alfabetu bez potrzeby cairo
    try:
        generate_idle_start_icon()
        generate_all_alphabet_letters()
    except Exception:
        pass

if __name__ == "__main__":
    generate_all_base_icons()
    print("Wygenerowano ujednolicone ostre ikony oraz alfabet.")
