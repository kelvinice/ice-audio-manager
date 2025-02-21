from src.gui.app import AudioManagerApp
import sys
from PyQt5.QtWidgets import QApplication

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = AudioManagerApp()
    window.show()
    sys.exit(app.exec_())