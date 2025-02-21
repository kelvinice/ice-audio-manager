import tkinter as tk
from tkinter import ttk
import pyaudio
import sounddevice as sd

class DeviceList:
    def __init__(self, master):
        self.master = master
        self.frame = ttk.Frame(master)
        self.frame.pack(padx=10, pady=10)

        self.label = ttk.Label(self.frame, text="Audio Devices")
        self.label.pack()

        self.tree = ttk.Treeview(self.frame)
        self.tree.pack()

        self.populate_device_list()

    def populate_device_list(self):
        self.tree.delete(*self.tree.get_children())
        self.tree["columns"] = ("Type", "Name")
        self.tree.column("#0", width=0, stretch=tk.NO)
        self.tree.column("Type", anchor=tk.W, width=100)
        self.tree.column("Name", anchor=tk.W, width=300)

        self.tree.heading("#0", text="", anchor=tk.W)
        self.tree.heading("Type", text="Type", anchor=tk.W)
        self.tree.heading("Name", text="Name", anchor=tk.W)

        p = pyaudio.PyAudio()
        for i in range(p.get_device_count()):
            device_info = p.get_device_info_by_index(i)
            device_type = "Input" if device_info['maxInputChannels'] > 0 else "Output"
            self.tree.insert("", "end", values=(device_type, device_info['name']))

        p.terminate()