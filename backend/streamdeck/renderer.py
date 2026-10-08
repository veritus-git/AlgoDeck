import io
from typing import Optional
from PIL import Image, ImageDraw, ImageFont

class ButtonRenderer:
    @staticmethod
    def render_key(
        label: str,
        icon: str = "",
        subtext: str = "",
        status: str = "IDLE",  # IDLE, RUNNING, OK, WA, TLE, RTE, CE, COPIED
        width: int = 72,
        height: int = 72
    ) -> Image.Image:
        """
        Renders a 72x72 RGB image for Stream Deck LCD keys with dynamic status colors,
        icon badge, and text labels.
        """
        # Determine background and border palette
        color_map = {
            "IDLE": ("#121620", "#283044", "#e2e8f0"),
            "RUNNING": ("#451a03", "#d97706", "#fef3c7"),
            "OK": ("#064e3b", "#10b981", "#d1fae5"),
            "WA": ("#450a0a", "#ef4444", "#fee2e2"),
            "TLE": ("#431407", "#f97316", "#ffedd5"),
            "RTE": ("#3b0764", "#a855f7", "#f3e8ff"),
            "CE": ("#450a0a", "#ef4444", "#fee2e2"),
            "COPIED": ("#082f49", "#0ea5e9", "#e0f2fe"),
            "DEBUG": ("#172554", "#3b82f6", "#dbeafe"),
            "ACTION": ("#1e1b4b", "#6366f1", "#e0e7ff"),
            "DANGER": ("#450a0a", "#dc2626", "#fee2e2")
        }

        bg_hex, border_hex, text_hex = color_map.get(status, color_map["IDLE"])

        img = Image.new("RGB", (width, height), bg_hex)
        draw = ImageDraw.Draw(img)

        # Draw rounded border
        draw.rounded_rectangle(
            [(2, 2), (width - 3, height - 3)],
            radius=8,
            outline=border_hex,
            width=2
        )

        # Simple default font rendering
        font_main = ImageFont.load_default()

        # 1. Top Icon / Symbol
        if icon:
            bbox = draw.textbbox((0, 0), icon, font=font_main)
            icon_w = bbox[2] - bbox[0]
            draw.text(((width - icon_w) // 2, 8), icon, fill=text_hex, font=font_main)

        # 2. Middle Label
        if label:
            # Shorten label if too long
            short_label = label[:8]
            bbox = draw.textbbox((0, 0), short_label, font=font_main)
            label_w = bbox[2] - bbox[0]
            draw.text(((width - label_w) // 2, 28), short_label, fill="#ffffff", font=font_main)

        # 3. Bottom Subtext / Status
        if subtext:
            short_sub = subtext[:9]
            bbox = draw.textbbox((0, 0), short_sub, font=font_main)
            sub_w = bbox[2] - bbox[0]
            # Draw badge background for status
            draw.text(((width - sub_w) // 2, 48), short_sub, fill=border_hex, font=font_main)

        return img

    @staticmethod
    def render_to_bytes(
        label: str,
        icon: str = "",
        subtext: str = "",
        status: str = "IDLE",
        width: int = 72,
        height: int = 72
    ) -> bytes:
        img = ButtonRenderer.render_key(label, icon, subtext, status, width, height)
        buffer = io.BytesIO()
        img.save(buffer, format="JPEG", quality=95)
        return buffer.getvalue()
