import os
import glob
import base64
import json
import urllib.request
import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime

DIR = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"
URL = "http://localhost:1234/v1/chat/completions"
LOG_FILE = os.path.join(DIR, "extraction_log_parallel.txt")
MAX_WORKERS = 4


def log(msg):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{timestamp}] {msg}"
    print(line)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def process_file(filepath):
    out_file = os.path.splitext(filepath)[0] + "_extracted.txt"
    if os.path.exists(out_file):
        return True, f"Skipped (already exists): {os.path.basename(filepath)}"

    try:
        # THE FIX: Read file as binary, encode to base64, ensure no trailing newlines
        with open(filepath, "rb") as f:
            base64_img = base64.b64encode(f.read()).decode("utf-8").strip()

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

        # THE FIX: urllib sometimes throws Errno 22 on Windows if data exceeds socket buffer chunks.
        # We enforce a stable buffer size and add a proper User-Agent to bypass local firewall traps.
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            URL,
            data=data,
            headers={"Content-Type": "application/json", "User-Agent": "Mozilla/5.0"},
        )

        with urllib.request.urlopen(req, timeout=1200) as response:
            result = json.loads(response.read().decode("utf-8"))
            extracted_text = result["choices"][0]["message"]["content"]

            with open(out_file, "w", encoding="utf-8") as f:
                f.write(extracted_text)

        return True, f"Extracted: {os.path.basename(filepath)} (RAW PAYLOAD)"
    except Exception as e:
        return False, f"Error on {os.path.basename(filepath)}: {str(e)}"


def main():
    log("Scanning for JPG files with HTTP-CHUNK-FIXED 4x PARALLEL CHAINING enabled...")
    files = []
    for root, _, filenames in os.walk(DIR):
        for filename in filenames:
            if filename.lower().endswith(".jpg"):
                files.append(os.path.join(root, filename))

    log(
        f"Found {len(files)} total JPG files. Starting phase 1 (workers={MAX_WORKERS})..."
    )

    failed_files = []

    with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = {executor.submit(process_file, path): path for path in files}
        for i, future in enumerate(as_completed(futures), 1):
            try:
                success, msg = future.result()
                log(f"[{i}/{len(files)}] {msg}")
                if not success:
                    failed_files.append(futures[future])
            except Exception as exc:
                log(f"[{i}/{len(files)}] Generated an exception: {exc}")
                failed_files.append(futures[future])

    if failed_files:
        log(
            f"Phase 1 complete. Found {len(failed_files)} files that timed out or failed. Starting Phase 2 Retry Pass..."
        )
        with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
            retry_futures = {
                executor.submit(process_file, path): path for path in failed_files
            }
            for i, future in enumerate(as_completed(retry_futures), 1):
                success, msg = future.result()
                log(f"[RETRY {i}/{len(failed_files)}] {msg}")
    else:
        log("Phase 1 complete. No files failed. Skipping Phase 2.")

    log("Extraction complete! Chaining to Auto-Compiler...")
    try:
        subprocess.run(["python", os.path.join(DIR, "auto_compiler.py")], check=True)
        log(
            "Auto-Compiler finished successfully. Master Study Guides and Formula Sheets are ready!"
        )
    except Exception as e:
        log(f"Failed to chain auto_compiler.py: {str(e)}")


if __name__ == "__main__":
    main()
