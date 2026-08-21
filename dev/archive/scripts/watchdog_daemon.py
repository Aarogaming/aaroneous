import time
import os
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import subprocess

DIR_TO_WATCH = (
    r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"
)


class ImageHandler(FileSystemEventHandler):
    def on_created(self, event):
        # We only care about new .jpg files
        if event.is_directory or not event.src_path.lower().endswith(".jpg"):
            return

        filepath = event.src_path
        print(f"New whiteboard image detected: {os.path.basename(filepath)}")

        # Wait a few seconds to ensure the file is completely saved/copied from your phone
        time.sleep(5)

        try:
            # Run the parallel_extract_resized.py script to process it
            subprocess.run(
                ["python", os.path.join(DIR_TO_WATCH, "parallel_extract_resized.py")],
                check=True,
            )
            print("Extraction complete.")

            # Immediately run the compiler to update the master study guides
            subprocess.run(
                ["python", os.path.join(DIR_TO_WATCH, "auto_compiler.py")], check=True
            )
            print("Master Study Guides updated with new notes!")

        except Exception as e:
            print(f"Error processing new image: {e}")


if __name__ == "__main__":
    event_handler = ImageHandler()
    observer = Observer()
    observer.schedule(event_handler, path=DIR_TO_WATCH, recursive=True)

    print(f"Watchdog active. Monitoring {DIR_TO_WATCH} for new .jpg files...")
    observer.start()

    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()
