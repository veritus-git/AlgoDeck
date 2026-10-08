import asyncio
import logging
import threading
from typing import Optional

from backend.config import settings
from backend.streamdeck.controller import StreamDeckController
from backend.streamdeck.renderer import ButtonRenderer

logger = logging.getLogger("algodeck.hardware")

class StreamDeckHardwareDriver:
    def __init__(self, controller: StreamDeckController):
        self.controller = controller
        self.deck = None
        self.loop = None
        self.is_connected = False

    def start(self, event_loop: asyncio.AbstractEventLoop):
        """Attempts to detect and connect to physical Stream Deck device."""
        if not settings.enable_streamdeck_hardware:
            logger.info("Stream Deck hardware support disabled in settings.")
            return

        self.loop = event_loop
        try:
            from StreamDeck.DeviceManager import DeviceManager
            from StreamDeck.ImageHelpers import PILHelper

            streamdecks = DeviceManager().enumerate()
            if not streamdecks:
                logger.info("No physical Elgato Stream Deck detected via USB. Virtual Web Deck is active.")
                return

            self.deck = streamdecks[0]
            self.deck.open()
            self.deck.reset()
            self.is_connected = True

            logger.info(f"Connected to physical Stream Deck: {self.deck.deck_type()} (Serial: {self.deck.get_serial_number()})")
            self.deck.set_brightness(80)

            # Register key callback
            self.deck.set_key_callback(self._on_hardware_key_press)

            # Subscribe to controller changes to update LCD images
            self.controller.add_listener(self._on_keys_state_updated)

            # Render initial state
            self._render_all_keys()

        except Exception as e:
            logger.warning(f"Could not initialize Stream Deck hardware: {e}. Virtual Web Deck will remain active.")

    def _on_hardware_key_press(self, deck, key_index: int, state: bool):
        """Hardware callback on button press (state=True means pressed down)."""
        if state and self.loop:
            logger.info(f"Physical Stream Deck key {key_index} pressed.")
            asyncio.run_coroutine_threadsafe(
                self.controller.execute_key(key_index),
                self.loop
            )

    def _on_keys_state_updated(self, keys_state):
        """Updates hardware LCD buttons when state changes."""
        if not self.deck or not self.is_connected:
            return
        self._render_all_keys()

    def _render_all_keys(self):
        if not self.deck:
            return
        from StreamDeck.ImageHelpers import PILHelper

        for key_data in self.controller.keys_state:
            idx = key_data["id"]
            if idx >= self.deck.key_count():
                break

            img = ButtonRenderer.render_key(
                label=key_data.get("label", ""),
                icon=key_data.get("icon", ""),
                subtext=key_data.get("subtext", ""),
                status=key_data.get("status", "IDLE")
            )
            # Convert to Stream Deck native format
            native_img = PILHelper.to_native_key_format(self.deck, img)
            try:
                self.deck.set_key_image(idx, native_img)
            except Exception as e:
                logger.error(f"Failed to set key image on hardware key {idx}: {e}")

    def close(self):
        if self.deck:
            try:
                self.deck.reset()
                self.deck.close()
            except Exception:
                pass
            self.is_connected = False
