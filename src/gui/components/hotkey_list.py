import customtkinter as ctk
import json
import os

class HotkeyListFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)
        self.config_file = "hotkeys.json"
        self.refresh_hotkeys()

    def refresh_hotkeys(self):
        # Clear existing widgets
        for widget in self.winfo_children():
            widget.destroy()

        if os.path.exists(self.config_file):
            with open(self.config_file, 'r') as f:
                self.hotkeys = json.load(f)

            # Create header
            header_frame = ctk.CTkFrame(self)
            header_frame.pack(fill="x", padx=5, pady=(0, 10))
            
            ctk.CTkLabel(header_frame, text="Device Name").pack(side="left")
            ctk.CTkLabel(header_frame, text="Actions").pack(side="right", padx=100)
            ctk.CTkLabel(header_frame, text="Hotkey").pack(side="right")

            # List all hotkeys
            for hotkey, data in self.hotkeys.items():
                item_frame = ctk.CTkFrame(self)
                item_frame.pack(fill="x", padx=5, pady=2)
                
                ctk.CTkLabel(item_frame, text=data['name']).pack(side="left")
                
                # Action buttons frame
                action_frame = ctk.CTkFrame(item_frame, fg_color="transparent")
                action_frame.pack(side="right")
                
                # Delete button
                delete_btn = ctk.CTkButton(
                    action_frame,
                    text="Delete",
                    width=60,
                    fg_color="red",
                    command=lambda h=hotkey: self.delete_hotkey(h)
                )
                delete_btn.pack(side="right", padx=5)
                
                # Edit button
                edit_btn = ctk.CTkButton(
                    action_frame,
                    text="Edit",
                    width=60,
                    command=lambda h=hotkey, d=data: self.edit_hotkey(h, d)
                )
                edit_btn.pack(side="right", padx=5)
                
                # Hotkey label
                ctk.CTkLabel(action_frame, text=hotkey.upper()).pack(side="right", padx=10)

    def delete_hotkey(self, hotkey):
        if hotkey in self.hotkeys:
            self.hotkeys.pop(hotkey)
            with open(self.config_file, 'w') as f:
                json.dump(self.hotkeys, f, indent=4)
            self.refresh_hotkeys()
                

    def edit_hotkey(self, hotkey, data):
        # Create edit dialog
        dialog = ctk.CTkInputDialog(
            text=f"Enter new hotkey for {data['name']}:",
            title="Edit Hotkey"
        )
        new_hotkey = dialog.get_input()
        
        if new_hotkey and new_hotkey.upper() != hotkey:
            # Remove old hotkey
            old_data = self.hotkeys.pop(hotkey)
            
            # Add new hotkey with updated data
            self.hotkeys[new_hotkey.upper()] = {
                'hotkey': new_hotkey.upper(),
                'name': old_data['name'],
                'index': old_data['index']
            }
            
            # Save changes
            with open(self.config_file, 'w') as f:
                json.dump(self.hotkeys, f, indent=4)
            
            self.refresh_hotkeys()