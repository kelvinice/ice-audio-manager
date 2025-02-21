import pyaudio
from PyQt5.QtWidgets import QWidget, QVBoxLayout, QTreeWidget, QTreeWidgetItem, QLabel

class DeviceList(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.initUI()
    
    def initUI(self):
        layout = QVBoxLayout(self)
        label = QLabel("Audio Devices")
        layout.addWidget(label)
        self.tree = QTreeWidget()
        self.tree.setColumnCount(2)
        self.tree.setHeaderLabels(["Type", "Name"])
        layout.addWidget(self.tree)
        self.populate_device_list()
    
    def populate_device_list(self):
        self.tree.clear()
        p = pyaudio.PyAudio()
        for i in range(p.get_device_count()):
            device_info = p.get_device_info_by_index(i)
            device_type = "Input" if device_info['maxInputChannels'] > 0 else "Output"
            item = QTreeWidgetItem([device_type, device_info['name']])
            self.tree.addTopLevelItem(item)
        p.terminate()