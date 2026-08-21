import os
from PIL import Image
from concurrent.futures import ThreadPoolExecutor, as_completed
import time

BASE_DIR = (
    r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"
)
OPT_DIR = os.path.join(BASE_DIR, "Optimized_Whiteboards")
MAX_PIXELS = 1024


def process_image(file_path):
    rel_path = os.path.relpath(file_path, BASE_DIR)
    opt_path = os.path.join(OPT_DIR, rel_path)

    txt_path = os.path.splitext(file_path)[0] + "_extracted.txt"
    if os.path.exists(txt_path) or os.path.exists(opt_path):
        return (
            f"Skipped (already extracted or optimized): {os.path.basename(file_path)}"
        )

    os.makedirs(os.path.dirname(opt_path), exist_ok=True)

    try:
        with Image.open(file_path) as img:
            if img.mode != "RGB":
                img = img.convert("RGB")
            img.thumbnail((MAX_PIXELS, MAX_PIXELS), Image.Resampling.LANCZOS)
            img.save(opt_path, format="JPEG", quality=85)
        return f"Optimized: {os.path.basename(file_path)}"
    except Exception as e:
        return f"Error resizing {file_path}: {e}"


def main():
    print(f"Scanning for original JPG files in {BASE_DIR}...")
    files_to_optimize = []

    for root, _, filenames in os.walk(BASE_DIR):
        if (
            "Optimized_Whiteboards" in root
            or "Notes" in root
            or "formula Sheets" in root
        ):
            continue
        for filename in filenames:
            if filename.lower().endswith(".jpg"):
                file_path = os.path.join(root, filename)
                # Check if it already has a txt file
                txt_path = os.path.splitext(file_path)[0] + "_extracted.txt"
                if not os.path.exists(txt_path):
                    files_to_optimize.append(file_path)

    print(
        f"Found {len(files_to_optimize)} remaining raw images to optimize. Unleashing 24-core CPU..."
    )

    start_time = time.time()
    with ThreadPoolExecutor(max_workers=20) as executor:
        futures = {
            executor.submit(process_image, path): path for path in files_to_optimize
        }
        for i, future in enumerate(as_completed(futures), 1):
            try:
                res = future.result()
                if i % 20 == 0 or i == len(files_to_optimize):
                    print(f"[{i}/{len(files_to_optimize)}] {res}")
            except Exception as e:
                print(f"[{i}/{len(files_to_optimize)}] Error: {e}")

    elapsed = time.time() - start_time
    print(
        f"Optimization complete! Took {elapsed:.1f} seconds. All images safely prepared for LM Studio."
    )


if __name__ == "__main__":
    main()
