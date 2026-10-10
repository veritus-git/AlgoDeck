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
SCALE = 4
SIZE_HI = 144 * SCALE  # 576
SIZE_FINAL = (144, 144)

def _create_canvas_hi():
    """Tworzy czarne płótno 4x wysokiej rozdzielczości (576x576) do supersamplingu."""
    img = Image.new("RGBA", (SIZE_HI, SIZE_HI), (0, 0, 0, 255))
    draw = ImageDraw.Draw(img)
    return img, draw

def _downsample(img: Image.Image) -> Image.Image:
    """Downsampling z 576x576 do 144x144 z użyciem filtru Lanczosa dla idealnej ostrości bez rozmycia."""
    return img.resize(SIZE_FINAL, resample=Image.Resampling.LANCZOS)

def generate_letter_icon(char: str) -> Path:
    """
    Pojedyncza duża, geometryczna litera (np. 'C', 'H', 'W')
    w CZYSTEJ BIELI (255, 255, 255) na całkowicie czarnym tle (#000000).
    """
    ICONS_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACE_FRONTEND.mkdir(parents=True, exist_ok=True)

    img, draw = _create_canvas_hi()
    clean_char = (char or "?").upper()[:1]
    font = _get_font(84 * SCALE)
    # Czysty, wyrazisty biały kolor
    draw.text((288, 272), clean_char, fill=(255, 255, 255, 255), font=font, anchor="mm")

    final_img = _downsample(img)

    filename = f"letter_{clean_char.lower()}.png"
    p1 = ICONS_DIR / filename
    p2 = WORKSPACE_FRONTEND / filename
    final_img.save(p1, format="PNG")
    final_img.save(p2, format="PNG")
    return p1

def generate_equals_icon() -> Image.Image:
    """Dwa poziome paski '=' na czarnym tle (#000000) wypełniające klawisze 1 i 5 górnego rzędu."""
    img, draw = _create_canvas_hi()
    c_white = (226, 232, 240, 255)
    draw.rounded_rectangle((160, 216, 416, 256), radius=16, fill=c_white)
    draw.rounded_rectangle((160, 320, 416, 360), radius=16, fill=c_white)
    return _downsample(img)

def generate_vscode_icon() -> Image.Image:
    """Czysty geometryczny symbol kodu: < / > na czarnym tle (#000000)."""
    img, draw = _create_canvas_hi()
    c_cyan = (56, 189, 248, 255)
    c_slash = (148, 163, 184, 255)
    
    # Lewy nawias <
    draw.line([(168, 288), (240, 192)], fill=c_cyan, width=32)
    draw.line([(168, 288), (240, 384)], fill=c_cyan, width=32)
    draw.ellipse([152, 272, 184, 304], fill=c_cyan)
    draw.ellipse([224, 176, 256, 208], fill=c_cyan)
    draw.ellipse([224, 368, 256, 400], fill=c_cyan)

    # Ukośnik /
    draw.line([(272, 400), (304, 176)], fill=c_slash, width=28)
    draw.ellipse([258, 386, 286, 414], fill=c_slash)
    draw.ellipse([290, 162, 318, 190], fill=c_slash)

    # Prawy nawias >
    draw.line([(408, 288), (336, 192)], fill=c_cyan, width=32)
    draw.line([(408, 288), (336, 384)], fill=c_cyan, width=32)
    draw.ellipse([392, 272, 424, 304], fill=c_cyan)
    draw.ellipse([320, 176, 352, 208], fill=c_cyan)
    draw.ellipse([320, 368, 352, 400], fill=c_cyan)

    return _downsample(img)

def generate_test_icon() -> Image.Image:
    """
    Ujednolicony geometryczny symbol testów: wyrazisty checkmark ✓
    o stałej grubości, z idealnie wypełnionym i zaokrąglonym narożnikiem bez wycięć.
    """
    img, draw = _create_canvas_hi()
    c_emerald = (16, 185, 129, 255)

    p1 = (144, 304)
    p2 = (240, 400)
    p3 = (436, 164)
    w = 56
    r = w // 2

    draw.line([p1, p2], fill=c_emerald, width=w)
    draw.line([p2, p3], fill=c_emerald, width=w)
    # Końcówki
    draw.ellipse([p1[0]-r, p1[1]-r, p1[0]+r, p1[1]+r], fill=c_emerald)
    draw.ellipse([p3[0]-r, p3[1]-r, p3[0]+r, p3[1]+r], fill=c_emerald)
    # Wierzchołek z pełnym promieniem miter (zerowe wycięcia czy braki px)
    draw.ellipse([p2[0]-38, p2[1]-38, p2[0]+38, p2[1]+38], fill=c_emerald)

    return _downsample(img)

def generate_play_icon() -> Image.Image:
    """Czysty, geometryczny zielony trójkąt Play ▶ na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_hi()
    c_green = (34, 197, 94, 255)
    pts = [(192, 150), (432, 288), (192, 426)]
    draw.polygon(pts, fill=c_green)
    return _downsample(img)

def generate_kill_icon() -> Image.Image:
    """Czysty, geometryczny czerwony kwadrat Stop ⏹ na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_hi()
    c_red = (239, 68, 68, 255)
    draw.rounded_rectangle((160, 160, 416, 416), radius=32, fill=c_red)
    return _downsample(img)

def generate_menu_icon() -> Image.Image:
    """Czyste, geometryczne 4 kwadraciki ⊞ (Zadania/Galeria) na czysto czarnym tle (#000000)."""
    img, draw = _create_canvas_hi()
    c_indigo = (129, 140, 248, 255)
    draw.rounded_rectangle((152, 152, 272, 272), radius=24, fill=c_indigo)
    draw.rounded_rectangle((304, 152, 424, 272), radius=24, fill=c_indigo)
    draw.rounded_rectangle((152, 304, 272, 424), radius=24, fill=c_indigo)
    draw.rounded_rectangle((304, 304, 424, 424), radius=24, fill=c_indigo)
    return _downsample(img)

def generate_arrow_left_icon() -> Image.Image:
    """Minimalistyczny szewron w lewo ‹ z idealnym, pełnym wierzchołkiem bez braków px."""
    img, draw = _create_canvas_hi()
    c_slate = (203, 213, 225, 255)
    draw.line([(352, 160), (224, 288)], fill=c_slate, width=38)
    draw.line([(224, 288), (352, 416)], fill=c_slate, width=38)
    draw.ellipse([224-27, 288-27, 224+27, 288+27], fill=c_slate)
    draw.ellipse([352-19, 160-19, 352+19, 160+19], fill=c_slate)
    draw.ellipse([352-19, 416-19, 352+19, 416+19], fill=c_slate)
    return _downsample(img)

def generate_arrow_right_icon() -> Image.Image:
    """Minimalistyczny szewron w prawo › z idealnym, pełnym wierzchołkiem bez braków px."""
    img, draw = _create_canvas_hi()
    c_slate = (203, 213, 225, 255)
    draw.line([(224, 160), (352, 288)], fill=c_slate, width=38)
    draw.line([(352, 288), (224, 416)], fill=c_slate, width=38)
    draw.ellipse([352-27, 288-27, 352+27, 288+27], fill=c_slate)
    draw.ellipse([224-19, 160-19, 224+19, 160+19], fill=c_slate)
    draw.ellipse([224-19, 416-19, 224+19, 416+19], fill=c_slate)
    return _downsample(img)

def generate_arrow_back_icon() -> Image.Image:
    """Minimalistyczna geometryczna strzałka powrotu ← z idealnym grotem bez braków px."""
    img, draw = _create_canvas_hi()
    c_slate = (203, 213, 225, 255)
    # Trzon
    draw.line([(168, 288), (416, 288)], fill=c_slate, width=38)
    # Skrzydła
    draw.line([(168, 288), (276, 180)], fill=c_slate, width=38)
    draw.line([(168, 288), (276, 396)], fill=c_slate, width=38)
    # Zaokrąglenia i pełny grot
    draw.ellipse([168-27, 288-27, 168+27, 288+27], fill=c_slate)
    draw.ellipse([416-19, 288-19, 416+19, 288+19], fill=c_slate)
    draw.ellipse([276-19, 180-19, 276+19, 180+19], fill=c_slate)
    draw.ellipse([276-19, 396-19, 276+19, 396+19], fill=c_slate)
    return _downsample(img)

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
    """Generuje komplet czystych geometrycznych ikon akcji na czarnym tle w 4x SSAA."""
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
        "icon_equals.png": generate_equals_icon(),
    }

    for name, img in icons_map.items():
        dst1 = ICONS_DIR / name
        dst2 = WORKSPACE_FRONTEND / name
        img.save(dst1, format="PNG")
        img.save(dst2, format="PNG")

    for name, img in icons_map.items():
        dst1 = ICONS_DIR / name
        dst2 = WORKSPACE_FRONTEND / name
        img.save(dst1, format="PNG")
        img.save(dst2, format="PNG")

if __name__ == "__main__":
    generate_all_base_icons()
    print("Wygenerowano geometryczne ikony na czarnym tle.")
