import customtkinter as ctk
import json
import os

class DeviceFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, devices, hotkey_controller, audio_controller, **kwargs):
        super().__init__(master, **kwargs)
        self.hotkey_controller = hotkey_controller
        self.audio_controller = audio_controller
        self.device_buttons = {}
        self.config_file = "hotkeys.json"  # Add this line
        self.load_hotkeys()  # Add this line before setup_devices
        self.setup_devices(devices)
        self.restore_hotkeys()

    def setup_devices(self, devices):
        for device in devices:
            device_frame = ctk.CTkFrame(self)
            device_frame.pack(fill="x", padx=5, pady=2)
            
            name_label = ctk.CTkLabel(device_frame, text=device['name'])
            name_label.pack(side="left")
            
            hotkey_label = ctk.CTkLabel(device_frame, text=self.get_saved_hotkey(device))
            hotkey_label.pack(side="left", padx=10)
            
            hotkey_btn = ctk.CTkButton(
                device_frame,
                text="Set Hotkey",
                command=lambda d=device, l=hotkey_label: self.record_hotkey(d, l)
            )
            hotkey_btn.pack(side="right")
            
            self.device_buttons[int(device['index'])] = (hotkey_btn, hotkey_label)

    def restore_hotkeys(self):
        if not hasattr(self, 'saved_hotkeys'):
            self.load_hotkeys()
        
        # Iterate through saved hotkeys and match by device index
        for hotkey_data in self.saved_hotkeys.values():
            device_index = int(hotkey_data['index'])
            if device_index in self.device_buttons:
                btn, label = self.device_buttons[device_index]
                label.configure(text=f"Hotkey: {hotkey_data['hotkey']}")
                
                # Re-register the hotkey
                device_info = self.audio_controller.get_device_by_index(device_index)
                if device_info:
                    self.hotkey_controller.register_hotkey(
                        hotkey_data['hotkey'],
                        lambda d=device_info: self.audio_controller.switch_to_device(d),
                        device_info
                    )

    def record_hotkey(self, device, label):
        btn, _ = self.device_buttons[device['index']]
        btn.configure(text="Recording...", state="disabled")
        
        def on_hotkey(hotkey):
            label.configure(text=f"Hotkey: {hotkey}")
            btn.configure(text="Set Hotkey", state="normal")
            
            # Save the hotkey first
            self.save_hotkey(device, hotkey)
            
            # Then register it
            self.hotkey_controller.register_hotkey(
                hotkey, 
                lambda: self.audio_controller.switch_to_device(device),
                {
                    'hotkey': hotkey,
                    'name': device['name'],
                    'index': device['index']
                }
            )
        
        self.hotkey_controller.start_recording(on_hotkey)

    def load_hotkeys(self):
        """Initialize saved_hotkeys from config file"""
        if os.path.exists(self.config_file):
            with open(self.config_file, 'r') as f:
                self.saved_hotkeys = json.load(f)
        else:
            self.saved_hotkeys = {}

    def save_hotkey(self, device, hotkey):
        # Use the hotkey as the key instead of device_id
        self.saved_hotkeys[hotkey] = {
            'hotkey': hotkey,
            'name': device['name'],
            'index': device['index']
        }
        with open(self.config_file, 'w') as f:
            json.dump(self.saved_hotkeys, f, indent=4)

    def get_saved_hotkey(self, device):
        # Search through hotkeys for matching device index
        for hotkey_data in self.saved_hotkeys.values():
            if hotkey_data['index'] == device['index']:
                return f"Hotkey: {hotkey_data['hotkey'].upper()}"
        return "No hotkey"