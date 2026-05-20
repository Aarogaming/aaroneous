from datetime import datetime, timedelta, timezone
import time as t
import json
import os
from dotenv import load_dotenv
import firebase_admin
from firebase_admin import credentials, db

# Initialize Firebase
load_dotenv()
DB_CREDENTIALS = json.loads(os.getenv("DB_CREDENTIALS"))
cred = credentials.Certificate(DB_CREDENTIALS)
firebase_admin.initialize_app(cred, {'databaseURL': DB_CREDENTIALS["databaseURL"]})

class Gardener:
    def __init__(self, actions, images, orders, locations, navigator):
        self.actions = actions
        self.Images = images
        self.Orders = orders
        self.Locations = locations
        self.navigator = navigator

    def garden(self, spell_order, garden, plant_info):
        # # Go to the garden
        print(f"Navigating to garden...")
        self.navigator.navigate_to_location(garden)

        # Handle plant needs
        if self.actions.is_image_visible(self.Images.HAS_NEEDS):
            print("Plants have needs! Handling needs...")
            self.cast_spell_order(spell_order)

        # If plants are ready to harvest, harvest them
        if self.actions.is_image_visible(self.Images.HARVEST):
            is_elder = self.harvest()

            if is_elder:
                print("Plants were elder! Replanting seeds...")
                self.replant_seeds(garden)

        state = self.determine_state()
        print(f"Current plant state: {state}")
        self.update_plant_state_in_db(state, plant_info)

    def cast_spell_order(self, spell_order, close_menu=True):
        self.actions.press_key("g")
        
        for step in spell_order:
            spell = step.get("spell")
            coordinates = step.get("coordinates")
            category = step.get("category")
            if not self.find_spell(spell, category):
                print(f"Failed to find spell: {spell}")
                return
            
            if not self.cast_spell(spell, coordinates):
                print(f"Failed to cast spell: {spell}")
                return
            
        if close_menu:
            self.actions.press_key("g")

    def find_spell(self, spell, category):
        category_image = getattr(self.Images, category, None)
        spell_image = getattr(self.Images, spell, None)
        
        print(f"Navigating to category: {category}...")
        if not self.actions.click_image(category_image):
            print(f"Failed to locate category: {category}.")
            return False
        
        print(f"Looking for spell: {spell}...")
        while not self.actions.is_image_visible(spell_image):
            print(f"{spell} not found on screen. Clicking arrow button...")
            if not self.actions.click_image(self.Images.GARDENING_RIGHT_ARROW):
                print("Failed to locate the arrow button.")
                return False
            t.sleep(0.5)
        
        print(f"Spell {spell} found")
        return True
    
    def cast_spell(self, spell, coordinates):
        spell_image = getattr(self.Images, spell, None)
        if not self.actions.click_image(spell_image):
            print(f"Failed to select spell: {spell}.")
            return False
        
        print(f"Spell {spell} selected. Casting spell...")
        if coordinates:
            self.actions.move_mouse_to(*coordinates, duration=0.5)
            t.sleep(5)
            self.actions.click()
        else:
            print(f"No coordinates specified for {spell}.")
        t.sleep(10)
        return True

    def harvest(self):
        print("Starting the harvesting process...")
        is_elder = self.actions.is_image_visible(self.Images.ELDER)
        self.navigator.navigate_to_location(self.Locations.HARVEST_SPOT)
        self.actions.hold_key("w")
        self.actions.hold_key("a")

        self.harvest_until_banner_gone()
        
        self.actions.release_key("w")
        self.actions.release_key("a")

        self.actions.press_key("a", 0.5)
        self.actions.press_key("w", 0.3)

        self.harvest_until_banner_gone()
        print("Harvesting process completed.")
        
        # Reset housing items
        self.actions.press_key("h")
        self.actions.press_key("h")
        return is_elder
    
    def harvest_until_banner_gone(self):
        harvest_absent_start = None
        while True:
            self.actions.press_key("x")
            t.sleep(0.1)

            if not self.actions.is_image_visible(self.Images.HARVEST):
                harvest_absent_start = harvest_absent_start or t.time()
                if t.time() - harvest_absent_start > 5:
                    break
            else:
                harvest_absent_start = None
        return
    
    def replant_seeds(self, garden):
        print("Replanting seeds...")

        # Navigate back to the garden and open the gardening menu
        self.navigator.go_to_start_of_house()
        self.navigator.navigate_to_location(garden)

        # Plant the first seed
        self.cast_spell_order(self.Orders.PLANT_ALL, close_menu=False)
        self.actions.click_image(self.Images.YES)


    def determine_state(self):
        self.actions.press_key("g")
        
        # Move the mouse to the popup location
        popup_coordinates = (1617, 537)
        self.actions.move_mouse_to(*popup_coordinates, duration=0.5)
        t.sleep(2)

        # Determine current state based on progress bar
        state = None
        if self.actions.is_image_visible(self.Images.PROGRESS_TO_YOUNG):
            state = "seedling"
        elif self.actions.is_image_visible(self.Images.PROGRESS_TO_MATURE):
            state = "young"
        elif self.actions.is_image_visible(self.Images.PROGRESS_TO_ELDER):
            state = "mature"
        else:
            print("Unable to determine the current state. No progress text found.")
            state = "unknown"
        self.actions.press_key("g")
        return state
    
    def update_plant_state_in_db(self, current_state, plant_info):
        try:
            # Fetch current state from the database
            ref = db.reference("plants/couch_potatoes")
            plant_state = ref.get()

            if not plant_state:
                print("Failed to fetch plant state from the database.")
                return

            # Check if the state needs updating
            db_current_state = plant_state.get("current_state")
            if current_state == db_current_state:
                print(f"No update needed. Current state in DB is already '{current_state}'.")
                return

            # Get the data for the update
            states = ["seedling", "young", "mature", "elder"]
            next_state = states[states.index(current_state) + 1] if current_state in states[:-1] else None
            transition_time = self.calculate_transition_time(current_state, plant_info)
            next_update_utc = datetime.now(timezone.utc) + timedelta(hours=transition_time)
            next_update = next_update_utc.astimezone(timezone(timedelta(hours=-5)))
            
            # Update the database
            ref.update({
                "current_state": current_state,
                "next_state": next_state,
                "next_update": next_update.isoformat(),
                "notified": False
            })
            print(f"Updated plant state in DB: current_state='{current_state}', next_state='{next_state}'")

        except Exception as e:
            print(f"Error updating plant state in DB: {e}")

    def calculate_transition_time(self, current_state, plant_info):
        transition_times = plant_info["state_durations"]
        likes = plant_info["likes"]

        # Base time for the current state
        base_time = transition_times.get(current_state, None)
        if base_time is None:
            print(f"Invalid current state: {current_state}")
            return None

        # Calculate the effective time with likes
        effective_time = base_time
        for like, boost in likes.items():
            if like == "pixie" and current_state != "mature":
                continue
            effective_time *= (1 - boost)

        print(f"Time to transition from '{current_state}' to the next state: {effective_time:.2f} hours")
        return effective_time
