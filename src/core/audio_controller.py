import pyaudio
import sounddevice as sd

class AudioController:
    def __init__(self):
        self.p = pyaudio.PyAudio()
        self._input_devices = []
        self._output_devices = []
        self.refresh_devices()

    def refresh_devices(self):
        self._input_devices.clear()
        self._output_devices.clear()
        
        for i in range(self.p.get_device_count()):
            device_info = self.p.get_device_info_by_index(i)
            if device_info['maxInputChannels'] > 0:
                self._input_devices.append(device_info)
            if device_info['maxOutputChannels'] > 0:
                self._output_devices.append(device_info)

    @property
    def input_devices(self):
        return self._input_devices

    @property
    def output_devices(self):
        return self._output_devices

    def switch_to_device(self, device_info):
        device_type = "Input" if device_info['maxInputChannels'] > 0 else "Output"
        print(f"Switching {device_type} device to: {device_info['name']} (ID: {device_info['index']})")
        # Here you would implement actual device switching logic

    def get_device_by_index(self, index):
        """Get device info by its index."""
        for device in self._input_devices + self._output_devices:
            if device['index'] == index:
                return device
        return None

    def __del__(self):
        self.p.terminate()