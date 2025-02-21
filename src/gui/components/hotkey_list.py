import json
import os
from PyQt5.QtWidgets import (
    QScrollArea,
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QLabel,
    QPushButton,
    QMessageBox,
    QInputDialog,
    QFrame
)
from PyQt5.QtCore import Qt

class HotkeyListFrame(QScrollArea):
    def __init__(self, hotkey_controller, parent=None):
        super().__init__(parent)
        self.hotkey_controller = hotkey_controller
        self.config_file = "hotkeys.json"
        self.refresh_hotkeys()
    
    def refresh_hotkeys(self):
        # Load hotkeys from file
        self.hotkeys = {}
        if os.path.exists(self.config_file):
            with open(self.config_file, 'r') as f:
                self.hotkeys = json.load(f)

        # Create main container widget
        container = QWidget()
        layout = QVBoxLayout(container)
        layout.setContentsMargins(5, 5, 5, 5)
        layout.setSpacing(10)

        # Header row
        header = QFrame()
        header_layout = QHBoxLayout(header)
        header_layout.setContentsMargins(5, 5, 5, 5)
        header_layout.setSpacing(10)
        name_header = QLabel("Device Name")
        hotkey_header = QLabel("Hotkey")
        actions_header = QLabel("Actions")
        header_layout.addWidget(name_header, 2)
        header_layout.addWidget(hotkey_header, 1)
        header_layout.addWidget(actions_header, 1, alignment=Qt.AlignRight)
        layout.addWidget(header)

        # Separator
        sep = QFrame()
        sep.setFrameShape(QFrame.HLine)
        layout.addWidget(sep)

        # Add a row for each hotkey
        for hotkey, data in self.hotkeys.items():
            row = QFrame()
            row_layout = QHBoxLayout(row)
            row_layout.setContentsMargins(5, 5, 5, 5)
            row_layout.setSpacing(10)

            name_label = QLabel(data['name'])
            hotkey_label = QLabel(hotkey.upper())

            # Actions widget with Edit and Delete buttons
            actions_widget = QWidget()
            actions_layout = QHBoxLayout(actions_widget)
            actions_layout.setContentsMargins(0, 0, 0, 0)
            actions_layout.setSpacing(5)
            edit_btn = QPushButton("Edit")
            edit_btn.setFixedWidth(60)
            delete_btn = QPushButton("Delete")
            delete_btn.setFixedWidth(60)
            edit_btn.clicked.connect(lambda _, h=hotkey, d=data: self.edit_hotkey(h, d))
            delete_btn.clicked.connect(lambda _, h=hotkey: self.delete_hotkey(h))
            actions_layout.addWidget(edit_btn)
            actions_layout.addWidget(delete_btn)

            row_layout.addWidget(name_label, 2)
            row_layout.addWidget(hotkey_label, 1)
            row_layout.addWidget(actions_widget, 1, alignment=Qt.AlignRight)

            # Add row to main layout
            layout.addWidget(row)

            # Add a separator between rows
            row_sep = QFrame()
            row_sep.setFrameShape(QFrame.HLine)
            layout.addWidget(row_sep)

        layout.addStretch()  # Push content to top

        self.setWidget(container)
        self.setWidgetResizable(True)
    
    def delete_hotkey(self, hotkey):
        if hotkey in self.hotkeys:
            reply = QMessageBox.question(
                self,
                "Confirm Delete",
                f"Are you sure you want to delete the hotkey: {hotkey}?",
                QMessageBox.Yes | QMessageBox.No
            )
            if reply == QMessageBox.Yes:
                self.hotkeys.pop(hotkey)
                with open(self.config_file, 'w') as f:
                    json.dump(self.hotkeys, f, indent=4)
                self.hotkey_controller.hotkeys.clear()
                self.hotkey_controller.load_hotkeys()
                self.refresh_hotkeys()
    
    def edit_hotkey(self, hotkey, data):
        new_hotkey, ok = QInputDialog.getText(
            self,
            "Edit Hotkey",
            f"Enter new hotkey for {data['name']}:"
        )
        if ok and new_hotkey and new_hotkey.upper() != hotkey:
            old_data = self.hotkeys.pop(hotkey)
            self.hotkeys[new_hotkey.upper()] = {
                'hotkey': new_hotkey.upper(),
                'name': old_data['name'],
                'index': old_data['index']
            }
            with open(self.config_file, 'w') as f:
                json.dump(self.hotkeys, f, indent=4)
            self.refresh_hotkeys()