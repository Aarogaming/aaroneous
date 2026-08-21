import os
import base64
import json
import requests
import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime

BASE_DIR = (
    r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"
)
OPT_DIR = os.path.join(BASE_DIR, "Optimized_Whiteboards")
URL = "http://localhost:1234/v1/chat/completions"
LOG_FILE = os.path.join(BASE_DIR, "extraction_log_parallel.txt")

# Safe sweet-spot for local vision servers processing TINY <0.1MB files
MAX_WORKERS = 6


def log(msg):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{timestamp}] {msg}"
    print(line)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def process_file(opt_path):
    # Map optimized path back to the original source path
    rel_path = os.path.relpath(opt_path, OPT_DIR)
    orig_path = os.path.join(BASE_DIR, rel_path)

    # Save the extracted text directly next to the original 7MB image in the original course folder
    out_file = os.path.splitext(orig_path)[0] + "_extracted.txt"
    if os.path.exists(out_file):
        return True, f"Skipped (already extracted): {os.path.basename(orig_path)}"

    try:
        # File is already highly compressed and 1024x1024, load instantly
        with open(opt_path, "rb") as f:
            base64_img = base64.b64encode(f.read()).decode("utf-8")

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

        response = requests.post(
            URL,
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=3600,
        )
        response.raise_for_status()

        extracted_text = response.json()["choices"][0]["message"]["content"]

        with open(out_file, "w", encoding="utf-8") as f:
            f.write(extracted_text)

        # Clean up the optimized temp file to save space
        try:
            os.remove(opt_path)
        except:
            pass

        return True, f"FAST Extracted: {os.path.basename(orig_path)}"
    except Exception as e:
        return False, f"Error on {os.path.basename(orig_path)}: {str(e)}"


def main():
    log("Scanning for OPTIMIZED JPG files. FAST DECOUPLED PIPELINE ACTIVE...")
    files = []
    for root, _, filenames in os.walk(OPT_DIR):
        for filename in filenames:
            if filename.lower().endswith(".jpg"):
                files.append(os.path.join(root, filename))

    log(f"Found {len(files)} tiny optimized JPGs. Starting (workers={MAX_WORKERS})...")

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
        log(f"Found {len(failed_files)} timed out files. Re-running...")
        with ThreadPoolExecutor(max_workers=MAX_WORKERS) as executor:
            retry_futures = {
                executor.submit(process_file, path): path for path in failed_files
            }
            for i, future in enumerate(as_completed(retry_futures), 1):
                success, msg = future.result()
                log(f"[RETRY {i}/{len(failed_files)}] {msg}")

    log("FAST Extraction complete! Chaining to Auto-Compiler...")
    try:
        subprocess.run(
            ["python", os.path.join(BASE_DIR, "auto_compiler.py")], check=True
        )
        log(
            "Auto-Compiler finished successfully. Master Study Guides and Formula Sheets are ready!"
        )
    except Exception as e:
        log(f"Failed to chain auto_compiler.py: {str(e)}")


if __name__ == "__main__":
    main()
