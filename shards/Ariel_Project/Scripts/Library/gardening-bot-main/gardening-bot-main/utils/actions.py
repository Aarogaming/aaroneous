import pyautogui
import mouse
import time

class Actions:
    def __init__(self):
        pass

    def press_key(self, key, duration=0.1):
        pyautogui.keyDown(key)
        time.sleep(duration)
        pyautogui.keyUp(key)

    def hold_key(self, key):
        pyautogui.keyDown(key)

    def release_key(self, key):
        pyautogui.keyUp(key)

    def perform_movements(self, movements):
        for movement in movements:
            key = movement.get("key")
            duration = movement.get("duration", 0)
            if key:
                print(f"Pressing {key} for {duration} seconds.")
                self.press_key(key, duration)

    def find_image(self, image_path, confidence=0.8, log_error=True):
        try:
            location = pyautogui.locateCenterOnScreen(image_path, confidence=confidence)
            return location
        except Exception as e:
            if log_error:
                print(f"Error while locating image: {e}")
            return None

    def is_image_visible(self, image_path, confidence=0.8):
        return self.find_image(image_path, confidence, log_error=False) is not None

    def wait_until_not_visible(self, image_path, confidence=0.8, timeout=30):
        start_time = time.time()
        while self.is_image_visible(image_path, confidence):
            if time.time() - start_time > timeout:
                print(f"Timeout reached while waiting for {image_path} to disappear.")
                return False
            time.sleep(0.5)
        return True

    def wait_until_visible(self, image_path, confidence=0.8, timeout=30):
        start_time = time.time()
        while not self.is_image_visible(image_path, confidence):
            if time.time() - start_time > timeout:
                print(f"Timeout reached while waiting for {image_path} to appear.")
                return False
            time.sleep(0.5)
        return True

    def move_mouse_to(self, x, y, duration=0.2):
        start_x, start_y = mouse.get_position()
        mouse.move(x - start_x, y - start_y, absolute=False, duration=duration)
        time.sleep(0.1)

    def click(self):
        mouse.click()

    def click_image(self, image_path, confidence=0.8):
        location = self.find_image(image_path, confidence)
        if location:
            self.move_mouse_to(location[0], location[1])
            self.click()
            time.sleep(0.5)
            return True
        else:
            print(f"Image not found: {image_path}")
            return False
