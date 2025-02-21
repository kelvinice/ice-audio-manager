import customtkinter as ctk
from core.audio_controller import AudioController
from core.hotkey_controller import HotkeyController
from gui.components.device_frame import DeviceFrame
from gui.components.hotkey_list import HotkeyListFrame
import json

class AudioManagerApp(ctk.CTk):
    def __init__(self):
        super().__init__()
        
        self.title("Ice Audio Manager")
        self.geometry("800x600")
        
        self.audio_controller = AudioController()
        self.hotkey_controller = HotkeyController()
        
        self.setup_ui()
        self.protocol("WM_DELETE_WINDOW", self.on_closing)

    def on_closing(self):
        """Save hotkeys before closing the application."""
        with open(self.hotkey_controller.config_file, 'w') as f:
            json.dump(self.hotkey_controller.hotkeys, f, indent=4)
        self.destroy()

    def refresh_hotkeys(self):
        """Refresh hotkeys across all components"""
        # Reset hotkey controller
        self.hotkey_controller.hotkeys.clear()
        self.hotkey_controller.load_hotkeys()
        
        # Rebuild the UI
        self.setup_ui()

    def setup_ui(self):
        # Create tabview
        self.tabview = ctk.CTkTabview(self)
        self.tabview.pack(fill="both", expand=True, padx=10, pady=10)
        
        # Add tabs
        self.tabview.add("Input Devices")
        self.tabview.add("Output Devices")
        self.tabview.add("Hotkeys")
        self.tabview.add("Settings")
        
        # Input devices tab
        input_frame = DeviceFrame(
            self.tabview.tab("Input Devices"),
            self.audio_controller.input_devices,
            self.hotkey_controller,
            self.audio_controller,
            width=750,
            height=500
        )
        input_frame.pack(fill="both", expand=True)
        
        # Output devices tab
        output_frame = DeviceFrame(
            self.tabview.tab("Output Devices"),
            self.audio_controller.output_devices,
            self.hotkey_controller,
            self.audio_controller,
            width=750,
            height=500
        )
        output_frame.pack(fill="both", expand=True)

        # Hotkeys tab
        hotkey_frame = HotkeyListFrame(
            self.tabview.tab("Hotkeys"),
            width=750,
            height=500
        )
        hotkey_frame.pack(fill="both", expand=True)
        
        # Settings tab
        settings_frame = ctk.CTkFrame(self.tabview.tab("Settings"))
        settings_frame.pack(fill="both", expand=True)
        
        refresh_btn = ctk.CTkButton(
            settings_frame,
            text="Refresh Devices",
            command=self.refresh_devices
        )
        refresh_btn.pack(pady=20)

    def refresh_devices(self):
        """Refresh only the devices data without rebuilding entire UI"""
        self.audio_controller.refresh_devices()
        # Update the device frames with new data
        if hasattr(self, 'input_frame'):
            self.input_frame.refresh_devices(self.audio_controller.input_devices)
        if hasattr(self, 'output_frame'):
            self.output_frame.refresh_devices(self.audio_controller.output_devices)