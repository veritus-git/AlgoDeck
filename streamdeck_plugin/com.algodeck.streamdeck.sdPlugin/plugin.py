#!/usr/bin/env python3
"""
AlgoDeck - Oficjalny Plugin Elgato Stream Deck (macOS & Windows)
Łączy oprogramowanie Elgato Stream Deck z lokalnym daemonem AlgoDeck (127.0.0.1:8080)
przez natywny protokół WebSocket SDK Elgato.
"""

import argparse
import asyncio
import json
import logging
import sys
import urllib.request
import urllib.error

logging.basicConfig(level=logging.INFO, format="%(asctime)s [AlgoDeck-Plugin] %(message)s")
logger = logging.getLogger("AlgoDeckPlugin")

API_BASE = "http://127.0.0.1:8080"

class AlgoDeckStreamDeckPlugin:
    def __init__(self, port: int, plugin_uuid: str, register_event: str, info: dict):
        self.port = port
        self.plugin_uuid = plugin_uuid
        self.register_event = register_event
        self.info = info
        self.ws = None
        self.task_contexts = set()

    async def run(self):
        try:
            import websockets
        except ImportError:
            # Fallback dla środowisk bez zewnętrznych pakietów: użycie asyncio streams lub urllib
            logger.warning("Brak biblioteki websockets, uruchamiam fallback HTTP bridge.")
            await self.run_http_fallback()
            return

        uri = f"ws://127.0.0.1:{self.port}"
        logger.info(f"Łączenie z Elgato Stream Deck pod {uri}...")

        async with websockets.connect(uri) as ws:
            self.ws = ws
            # Rejestracja pluginu
            reg_msg = {
                "event": self.register_event,
                "uuid": self.plugin_uuid
            }
            await ws.send(json.dumps(reg_msg))
            logger.info("Zarejestrowano plugin w Elgato Stream Deck.")

            # Uruchomienie pętli odświeżania aktywnego zadania
            asyncio.create_task(self.poll_active_task_loop())

            # Nasłuchiwanie zdarzeń od Stream Decka
            async for message in ws:
                try:
                    data = json.loads(message)
                    await self.handle_event(data)
                except Exception as e:
                    logger.error(f"Błąd przetwarzania zdarzenia: {e}")

    async def handle_event(self, data: dict):
        event = data.get("event")
        action = data.get("action", "")
        context = data.get("context", "")

        if event == "willAppear":
            if "task" in action:
                self.task_contexts.add(context)
                await self.update_task_badge(context)

        elif event == "willDisappear":
            if context in self.task_contexts:
                self.task_contexts.remove(context)

        elif event == "keyDown":
            await self.handle_action_press(action, context)

    async def handle_action_press(self, action: str, context: str):
        logger.info(f"Naciśnięto klawisz: {action}")
        try:
            if "test" in action:
                self.http_post(f"{API_BASE}/api/run-all-active")
            elif "run" in action:
                self.http_post(f"{API_BASE}/api/run-active")
            elif "kill" in action:
                self.http_post(f"{API_BASE}/api/kill-active")
            elif "next" in action:
                self.http_post(f"{API_BASE}/api/switch-task/next")
            elif "prev" in action:
                self.http_post(f"{API_BASE}/api/switch-task/prev")
            elif "settings" in action:
                self.http_post(f"{API_BASE}/api/settings/open-gui")
            elif "exit" in action:
                self.http_post(f"{API_BASE}/api/streamdeck/idle")
        except Exception as e:
            logger.error(f"Błąd wywołania API: {e}")

    async def update_task_badge(self, context: str):
        if not self.ws:
            return
        active_id = "ALGO"
        try:
            req = urllib.request.Request(f"{API_BASE}/api/status")
            with urllib.request.urlopen(req, timeout=1) as res:
                if res.status == 200:
                    data = json.loads(res.read().decode("utf-8"))
                    active_id = (data.get("active_problem") or "ALGO").upper()
        except Exception:
            pass

        title = f"● {active_id}" if active_id != "ALGO" else "● ZAD"
        payload = {
            "event": "setTitle",
            "context": context,
            "payload": {
                "title": title,
                "target": 0
            }
        }
        await self.ws.send(json.dumps(payload))

    async def poll_active_task_loop(self):
        while True:
            await asyncio.sleep(2.0)
            for ctx in list(self.task_contexts):
                try:
                    await self.update_task_badge(ctx)
                except Exception:
                    pass

    def http_post(self, url: str):
        try:
            req = urllib.request.Request(url, method="POST")
            urllib.request.urlopen(req, timeout=2)
        except Exception as e:
            logger.warning(f"Błąd HTTP POST do {url}: {e}")

    async def run_http_fallback(self):
        logger.info("Uruchomiono tryb demonstracyjny Elgato Stream Deck.")
        while True:
            await asyncio.sleep(10.0)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-port", type=int, default=0)
    parser.add_argument("-pluginUUID", type=str, default="")
    parser.add_argument("-registerEvent", type=str, default="")
    parser.add_argument("-info", type=str, default="{}")
    args = parser.parse_args()

    try:
        info_dict = json.loads(args.info)
    except Exception:
        info_dict = {}

    plugin = AlgoDeckStreamDeckPlugin(args.port, args.pluginUUID, args.registerEvent, info_dict)
    asyncio.run(plugin.run())

if __name__ == "__main__":
    main()
