import sys
import json
from PyQt5.QtWidgets import QMainWindow, QApplication, QTabWidget, QWidget, QVBoxLayout, QPushButton
from core.audio_controller import AudioController
from core.hotkey_controller import HotkeyController
from gui.components.device_frame import DeviceFrame
from gui.components.hotkey_list import HotkeyListFrame

class AudioManagerApp(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Ice Audio Manager")
        self.resize(800, 600)
        self.audio_controller = AudioController()
        self.hotkey_controller = HotkeyController()
        self.initUI()
    
    def initUI(self):
        self.tab_widget = QTabWidget()
        self.setCentralWidget(self.tab_widget)
        
        # Create tabs
        self.input_tab = QWidget()
        self.output_tab = QWidget()
        self.hotkeys_tab = QWidget()
        self.settings_tab = QWidget()
        
        self.tab_widget.addTab(self.input_tab, "Input Devices")
        self.tab_widget.addTab(self.output_tab, "Output Devices")
        self.tab_widget.addTab(self.hotkeys_tab, "Hotkeys")
        self.tab_widget.addTab(self.settings_tab, "Settings")
        
        # Setup tabs
        self.setupInputTab()
        self.setupOutputTab()
        self.setupHotkeysTab()
        self.setupSettingsTab()
    
    def setupInputTab(self):
        layout = QVBoxLayout()
        self.input_frame = DeviceFrame(
            self.audio_controller.input_devices,
            self.hotkey_controller,
            self.audio_controller
        )
        layout.addWidget(self.input_frame)
        self.input_tab.setLayout(layout)
    
    def setupOutputTab(self):
        layout = QVBoxLayout()
        self.output_frame = DeviceFrame(
            self.audio_controller.output_devices,
            self.hotkey_controller,
            self.audio_controller
        )
        layout.addWidget(self.output_frame)
        self.output_tab.setLayout(layout)
    
    def setupHotkeysTab(self):
        layout = QVBoxLayout()
        self.hotkey_list_frame = HotkeyListFrame(self.hotkey_controller)
        layout.addWidget(self.hotkey_list_frame)
        self.hotkeys_tab.setLayout(layout)
    
    def setupSettingsTab(self):
        layout = QVBoxLayout()
        refresh_btn = QPushButton("Refresh Devices")
        refresh_btn.clicked.connect(self.refresh_devices)
        layout.addWidget(refresh_btn)
        self.settings_tab.setLayout(layout)
    
    def refresh_devices(self):
        self.audio_controller.refresh_devices()
        # Update device frames with new data
        self.input_frame.update_devices(self.audio_controller.input_devices)
        self.output_frame.update_devices(self.audio_controller.output_devices)
    
    def closeEvent(self, event):
        with open(self.hotkey_controller.config_file, 'w') as f:
            json.dump(self.hotkey_controller.hotkeys, f, indent=4)
        event.accept()