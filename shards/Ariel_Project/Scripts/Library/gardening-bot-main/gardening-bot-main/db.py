import json
import os
from dotenv import load_dotenv
import firebase_admin
from firebase_admin import credentials, db

load_dotenv()

# Initialize Firebase
DB_CREDENTIALS = json.loads(os.getenv("DB_CREDENTIALS"))
cred = credentials.Certificate(DB_CREDENTIALS)
firebase_admin.initialize_app(cred, {'databaseURL': DB_CREDENTIALS["databaseURL"]})

# Write Data to Firebase
def update_plant_state(new_state, next_state, next_update, notified):
    ref = db.reference("plants/couch_potatoes")
    ref.set({
        "current_state": new_state,
        "next_update": next_update,
        "next_state": next_state,
        "notified": notified
    })
    print("Plant state updated successfully!")

# Read Data from Firebase
def get_plant_state():
    ref = db.reference("plants/couch_potatoes")
    return ref.get()

# Example Usage
if __name__ == "__main__":
    update_plant_state("mature", "elder", "2025-01-21T11:20:00", False)
    state = get_plant_state()
    print("Current Plant State:", state)
