import os
import io
import glob
import base64
import json
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from PIL import Image

DIR = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"
URL = "http://localhost:1234/v1/chat/completions"
LOG_FILE = os.path.join(DIR, "extraction_log_parallel.txt")
MAX_WORKERS = 4  # Matches your LM Studio parallel setting
MAX_PIXELS = 1920  # Max HD resolution (1080p width/height)


def log(msg):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{timestamp}] {msg}"
    print(line)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def process_file(filepath):
    out_file = os.path.splitext(filepath)[0] + "_extracted.txt"
    if os.path.exists(out_file):
        return f"Skipped (already exists): {os.path.basename(filepath)}"

    try:
        # Check file size (if > 1.5MB, auto-compress/resize)
        file_size = os.path.getsize(filepath)
        if file_size > 1.5 * 1024 * 1024:
            with Image.open(filepath) as img:
                # Convert to RGB if needed to prevent saving errors
                if img.mode != "RGB":
                    img = img.convert("RGB")
                # Resize while maintaining aspect ratio (Lanczos is high quality)
                img.thumbnail((MAX_PIXELS, MAX_PIXELS), Image.Resampling.LANCZOS)

                # Save to an in-memory buffer instead of disk
                buffer = io.BytesIO()
                img.save(buffer, format="JPEG", quality=85)
                base64_img = base64.b64encode(buffer.getvalue()).decode("utf-8")

                # Log that it was dynamically compressed
                old_mb = file_size / (1024 * 1024)
                new_mb = len(buffer.getvalue()) / (1024 * 1024)
                compress_msg = f" (compressed {old_mb:.1f}MB -> {new_mb:.1f}MB)"
        else:
            with open(filepath, "rb") as f:
                base64_img = base64.b64encode(f.read()).decode("utf-8")
            compress_msg = ""

        payload = {
            "model": "local-model",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": "You are a technical document extractor. Extract all mathematical formulas, equations, circuit details, circuit values, and theoretical concepts from this whiteboard image. Format formulas cleanly using markdown math notation where appropriate. Do NOT describe the image visually (e.g. 'The image shows...'). Just output the raw technical information, rules, concepts, and formulas.",
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": f"data:image/jpeg;base64,{base64_img}"
                            },
                        },
                    ],
                }
            ],
            "temperature": 0.1,
            "max_tokens": 2000,
        }

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            URL, data=data, headers={"Content-Type": "application/json"}
        )

        with urllib.request.urlopen(req, timeout=1200) as response:
            result = json.loads(response.read().decode("utf-8"))
            extracted_text = result["choices"][0]["message"]["content"]

            with open(out_file, "w", encoding="utf-8") as f:
                f.write(extracted_text)

        return f"Extracted: {os.path.basename(filepath)}{compress_msg}"
    except Exception as e:
        return f"Error on {os.path.basename(filepath)}: {str(e)}"


def main():
    log("Scanning for JPG files with auto-resize enabled...")
    files = []
    for root, _, filenames in os.walk(DIR):
        for filename in filenames:
            if filename.lower().endswith(".jpg"):
                files.append(os.path.join(root, filename))

    log(
        f"Found {len(files)} total JPG files. Starting resized parallel processing (workers={MAX_WORKERS})..."
    )

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = {executor.submit(process_file, path): path for path in files}
        for i, future in enumerate(as_completed(futures), 1):
            try:
                res = future.result()
                log(f"[{i}/{len(files)}] {res}")
            except Exception as exc:
                log(f"[{i}/{len(files)}] Generated an exception: {exc}")


if __name__ == "__main__":
    main()
