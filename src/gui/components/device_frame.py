import json
import os
from PyQt5.QtWidgets import QScrollArea, QWidget, QVBoxLayout, QHBoxLayout, QLabel, QPushButton, QFrame
from PyQt5.QtCore import Qt

class DeviceFrame(QScrollArea):
    def __init__(self, devices, hotkey_controller, audio_controller, parent=None):
        super().__init__(parent)
        self.hotkey_controller = hotkey_controller
        self.audio_controller = audio_controller
        self.devices = devices
        self.device_buttons = {}
        self.config_file = "hotkeys.json"
        self.load_hotkeys()
        self.initUI()
    
    def initUI(self):
        self.widget = QWidget()
        self.layout = QVBoxLayout(self.widget)
        self.widget.setLayout(self.layout)
        self.setWidget(self.widget)
        self.setWidgetResizable(True)
        self.setup_devices(self.devices)
        self.restore_hotkeys()
    
    def setup_devices(self, devices):
        # Clear any existing content
        while self.layout.count():
            child = self.layout.takeAt(0)
            if child.widget():
                child.widget().deleteLater()
        
        # Header row
        header = QFrame()
        header_layout = QHBoxLayout(header)
        header_layout.setContentsMargins(5, 5, 5, 5)
        name_header = QLabel("Device Name")
        hotkey_header = QLabel("Hotkey")
        actions_header = QLabel("Actions")
        header_layout.addWidget(name_header, 50)
        header_layout.addWidget(hotkey_header, 30)
        header_layout.addWidget(actions_header, 20, alignment=Qt.AlignRight)
        self.layout.addWidget(header)
        
        # Separator
        sep = QFrame()
        sep.setFrameShape(QFrame.HLine)
        self.layout.addWidget(sep)
        
        # Device rows
        self.device_buttons = {}
        for device in devices:
            row = QFrame()
            row_layout = QHBoxLayout(row)
            row_layout.setContentsMargins(5, 5, 5, 5)
            name_label = QLabel(device['name'])
            hotkey_label = QLabel(self.get_saved_hotkey(device))
            set_hotkey_btn = QPushButton("Set Hotkey")
            # Use a lambda to pass the current device and label
            set_hotkey_btn.clicked.connect(lambda _, d=device, l=hotkey_label: self.record_hotkey(d, l))
            
            row_layout.addWidget(name_label, 50)
            row_layout.addWidget(hotkey_label, 30)
            row_layout.addWidget(set_hotkey_btn, 20, alignment=Qt.AlignRight)
            self.layout.addWidget(row)
            
            self.device_buttons[int(device['index'])] = (set_hotkey_btn, hotkey_label)
    
    def restore_hotkeys(self):
        if not hasattr(self, 'saved_hotkeys'):
            self.load_hotkeys()
        for hotkey_data in self.saved_hotkeys.values():
            device_index = int(hotkey_data['index'])
            if device_index in self.device_buttons:
                btn, label = self.device_buttons[device_index]
                label.setText(hotkey_data['hotkey'])
                device_info = self.audio_controller.get_device_by_index(device_index)
                if device_info:
                    self.hotkey_controller.register_hotkey(
                        hotkey_data['hotkey'],
                        lambda d=device_info: self.audio_controller.switch_to_device(d),
                        device_info
                    )
    
    def record_hotkey(self, device, label):
        btn, _ = self.device_buttons[device['index']]
        btn.setText("Recording...")
        btn.setEnabled(False)
        
        def on_hotkey(hotkey):
            label.setText(f"{hotkey}")
            btn.setText("Set Hotkey")
            btn.setEnabled(True)
            self.save_hotkey(device, hotkey)
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
        if os.path.exists(self.config_file):
            with open(self.config_file, 'r') as f:
                self.saved_hotkeys = json.load(f)
        else:
            self.saved_hotkeys = {}
    
    def save_hotkey(self, device, hotkey):
        self.saved_hotkeys[hotkey] = {
            'hotkey': hotkey,
            'name': device['name'],
            'index': device['index']
        }
        with open(self.config_file, 'w') as f:
            json.dump(self.saved_hotkeys, f, indent=4)
    
    def get_saved_hotkey(self, device):
        for hotkey_data in self.saved_hotkeys.values():
            if hotkey_data['index'] == device['index']:
                return hotkey_data['hotkey'].upper()
        return "No hotkey"
    
    def update_devices(self, devices):
        self.devices = devices
        self.setup_devices(devices)
        self.restore_hotkeys()