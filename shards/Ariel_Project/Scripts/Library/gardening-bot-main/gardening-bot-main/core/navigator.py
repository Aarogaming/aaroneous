import time as t

class Navigator:
    def __init__(self, actions, images, locations):
        self.actions = actions
        self.Images = images
        self.Locations = locations
        self.current_location = None

    def go_home(self):
        if self.actions.is_image_visible(self.Images.HOUSING_MENU):
            print("Player is already at home.")
            return

        print("Attempting to go to the player's home...")
        if self.actions.click_image(self.Images.HOME):
            print("Clicked on the 'home' button.")
            self.current_location = None
            self.wait_for_loading_screen()
        else:
            print("Failed to locate the 'home' button.")

    def wait_for_loading_screen(self):
        print("Waiting for the loading screen to start...")
        if self.actions.wait_until_not_visible(self.Images.SPELLBOOK):
            print("Loading screen has started.")
        else:
            print("Timeout while waiting for the loading screen to start.")
            return

        print("Waiting for the loading screen to finish...")
        if self.actions.wait_until_visible(self.Images.SPELLBOOK):
            print("Loading screen completed.")
            t.sleep(1)
        else:
            print("Timeout while waiting for the loading screen to complete.")

    def navigate_to_location(self, location_sequence):
        if location_sequence:
            print(f"Navigating to location...")
            self.actions.perform_movements(location_sequence)
            self.current_location = location_sequence
            print("Arrived at the location.")
        else:
            print("No movement sequence provided.")

    def go_to_start_of_house(self):
        self.actions.press_key("h")
        self.actions.click_image(self.Images.TELEPORT_START)
        self.actions.click_image(self.Images.OUTSIDE)
        self.actions.press_key("h")
        self.current_location = None
        t.sleep(3)
