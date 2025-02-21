import comtypes
from comtypes import CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED

# Initialize COM in the main thread before any other imports
CoInitializeEx(COINIT_APARTMENTTHREADED)

import pyaudio
import comtypes.client as cc
from ctypes import HRESULT, c_int, c_wchar_p
from comtypes import GUID, COMMETHOD
from pycaw.pycaw import AudioUtilities

class IPolicyConfig(comtypes.IUnknown):
    _iid_ = GUID("{f8679f50-850a-41cf-9c72-430f290290c8}")
    _methods_ = [
        COMMETHOD([], HRESULT, "SetDefaultEndpoint",
                  (['in'], c_wchar_p, "wszDeviceId"),
                  (['in'], c_int, "eRole"))
    ]

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

    def get_device_by_index(self, index):
        for device in self._input_devices + self._output_devices:
            if device['index'] == index:
                return device
        return None

    def get_device_id_by_name(self, device_name):
        devices = AudioUtilities.GetAllDevices()
        for dev in devices:
            if device_name in dev.FriendlyName:
                return dev.id
        return None

    def switch_to_device(self, device_info):
        device_type = "Input" if device_info['maxInputChannels'] > 0 else "Output"
        print(f"Switching {device_type} device to: {device_info['name']}")

        device_id = self.get_device_id_by_name(device_info['name'])
        if not device_id:
            print(f"Device ID not found for {device_info['name']}")
            return False

        try:
            CLSID_PolicyConfigClient = GUID("{870af99c-171d-4f9e-af0d-e63df40c2bc9}")
            policy_config = cc.CreateObject(CLSID_PolicyConfigClient, interface=IPolicyConfig)
            for role in range(3):  # Try all roles (Console, Multimedia, Communications)
                hr = policy_config.SetDefaultEndpoint(device_id, role)
                if hr != 0:
                    print(f"HRESULT {hr} for role {role}")
            return True
        except Exception as e:
            print(f"Error: {e}")
            return False

    def __del__(self):
        self.p.terminate()
