import pynput

from pynput.mouse import Controller as MouseController
from pynput.keyboard import Listener as KeyboardListener, Key

# Initialize mouse controller
mouse = MouseController()

def on_press(key):
    try:
        # If the '\' key is pressed
        if key.char == '\\':
            x, y = mouse.position
            print(f"Mouse coordinates: x={x}, y={y}")

    except AttributeError:
        # Handle special keys
        if key == Key.esc:
            print("Exiting program...")
            return False  # Stop the listener

def main():
    print("Press '\\' to get mouse coordinates. Press 'Esc' to exit.")

    # Start listening to keyboard events
    with KeyboardListener(on_press=on_press) as listener:
        listener.join()

if __name__ == "__main__":
    main()
