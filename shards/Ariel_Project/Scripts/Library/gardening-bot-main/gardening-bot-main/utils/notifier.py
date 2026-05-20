import os
import json
import smtplib
from email.mime.text import MIMEText
from datetime import datetime, timezone, timedelta
from firebase_admin import credentials, db, initialize_app
from dotenv import load_dotenv

load_dotenv()

# Load environment variables
FROM_EMAIL = os.getenv("FROM_EMAIL")
FROM_PASSWORD = os.getenv("FROM_PASSWORD")
TO_EMAIL = os.getenv("TO_EMAIL")
SMTP_SERVER = os.getenv("SMTP_SERVER", "smtp.gmail.com")
SMTP_PORT = int(os.getenv("SMTP_PORT", 587))
DB_CREDENTIALS = json.loads(os.getenv("DB_CREDENTIALS"))

# Initialize Firebase
cred = credentials.Certificate(DB_CREDENTIALS)
initialize_app(cred, {'databaseURL': DB_CREDENTIALS["databaseURL"]})


def send_email(subject, body):
    try:
        msg = MIMEText(body)
        msg["Subject"] = subject
        msg["From"] = FROM_EMAIL
        msg["To"] = TO_EMAIL

        with smtplib.SMTP(SMTP_SERVER, SMTP_PORT) as server:
            server.starttls()
            server.login(FROM_EMAIL, FROM_PASSWORD)
            server.send_message(msg)

        print("Notification sent successfully!")

    except Exception as e:
        print(f"Failed to send email: {e}")


def fetch_plant_state():
    try:
        ref = db.reference("plants/couch_potatoes")
        return ref.get()
    except Exception as e:
        print(f"Error fetching plant state: {e}")
        return None


def update_notification_status():
    try:
        ref = db.reference("plants/couch_potatoes/notified")
        ref.set(True)
        print("Notification status updated to True.")
    except Exception as e:
        print(f"Error updating notification status: {e}")


def check_and_notify():
    plant_state = fetch_plant_state()
    if not plant_state:
        return
    print(f"Plant state: {plant_state}")

    next_update = datetime.fromisoformat(plant_state["next_update"])
    now = datetime.now(timezone(timedelta(hours=-5)))

    # Check if we are ready to send the notification
    print(f"Current Time: {now}")
    print(f"Next Update at: {next_update}")
    
    # Check if the notification has already been sent
    if plant_state.get("notified", False):
        print("Notification already sent for this state. Skipping.")
        return
    
    if now < next_update:
        print("Plants are not ready to be checked on.")
        return

    subject = f"Couch Potatoes Transitioned to {plant_state['next_state'].capitalize()}!"
    body = (
        f"Your couch potatoes have transitioned to the '{plant_state['next_state']}' stage.\n"
        "Please take care of them in Wizard101!"
    )
    send_email(subject, body)
    update_notification_status()


if __name__ == "__main__":
    check_and_notify()
