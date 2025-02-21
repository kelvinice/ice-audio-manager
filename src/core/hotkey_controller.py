import keyboard
from threading import Lock
import json
import os

class HotkeyController:
    def __init__(self):
        self.hotkeys = {}
        self.recording = False
        self.lock = Lock()
        self._current_keys = set()
        self._callback = None
        self.config_file = "hotkeys.json"
        self.load_hotkeys()

    def load_hotkeys(self):
        if os.path.exists(self.config_file):
            with open(self.config_file, 'r') as f:
                saved_hotkeys = json.load(f)
                for hotkey, data in saved_hotkeys.items():
                    print(f"Loading hotkey: {data['hotkey']} for device: {data['name']}")
                    self.hotkeys[hotkey] = {
                        'hotkey': data['hotkey'],
                        'name': data['name'],
                        'index': data['index']
                    }

    def start_recording(self, callback):
        with self.lock:
            self.recording = True
            self._current_keys.clear()
            self._callback = callback
            keyboard.hook(self._on_key_event)

    def stop_recording(self):
        with self.lock:
            self.recording = False
            keyboard.unhook_all()
            if self._current_keys:
                # Convert hotkey to uppercase when creating
                hotkey = '+'.join(sorted(key.upper() for key in self._current_keys))
                if self._callback:
                    self._callback(hotkey)
            self._current_keys.clear()
            self._callback = None

    def register_hotkey(self, hotkey, callback, device_info):
        keyboard.add_hotkey(hotkey, callback)
        self.hotkeys[hotkey] = {
            'hotkey': hotkey,
            'name': device_info['name'],
            'index': device_info['index']
        }
        print(f"Registered hotkey: {hotkey} for device: {device_info['name']}")

    def _on_key_event(self, event):
        if not self.recording:
            return
        if event.event_type == keyboard.KEY_DOWN:
            # Convert key name to uppercase when adding
            self._current_keys.add(event.name.upper())
        elif event.event_type == keyboard.KEY_UP:
            # Convert to uppercase for comparison
            if event.name.upper() in self._current_keys:
                self.stop_recording()