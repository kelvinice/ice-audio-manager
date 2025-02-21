class HotkeyManager:
    def __init__(self):
        self.hotkeys = {}

    def register_hotkey(self, key, action):
        if key in self.hotkeys:
            print(f"Hotkey '{key}' is already registered.")
        else:
            self.hotkeys[key] = action
            print(f"Hotkey '{key}' registered for action: {action}")

    def unregister_hotkey(self, key):
        if key in self.hotkeys:
            del self.hotkeys[key]
            print(f"Hotkey '{key}' unregistered.")
        else:
            print(f"Hotkey '{key}' not found.")

    def execute_hotkey(self, key):
        if key in self.hotkeys:
            action = self.hotkeys[key]
            print(f"Executing action for hotkey '{key}': {action}")
            # Here you would call the actual function associated with the hotkey
        else:
            print(f"No action registered for hotkey '{key}'.")