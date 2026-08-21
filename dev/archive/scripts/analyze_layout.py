import base64
import requests
import json
import os

IMAGE_PATH = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College\PXL_20260327_002947933.RAW-01.jpg"
if not os.path.exists(IMAGE_PATH):
    # Try finding it in ETE 120 or ETM 110
    base = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"
    for root, _, files in os.walk(base):
        if "PXL_20260327_002947933.RAW-01.jpg" in files:
            IMAGE_PATH = os.path.join(root, "PXL_20260327_002947933.RAW-01.jpg")
            break

try:
    with open(IMAGE_PATH, "rb") as f:
        base64_img = base64.b64encode(f.read()).decode("utf-8")

    payload = {
        "model": "local-model",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "Describe the arrangement of the math formulas on the page. Specifically: 1) What shape is formed on the left? 2) What is in the middle? 3) What is on the right? 4) What is at the top vs bottom? 5) Where do the parallel lines go relative to the Triangle and the Y? Keep it under 100 words.",
                    },
                    {
                        "type": "image_url",
                        "image_url": {"url": f"data:image/jpeg;base64,{base64_img}"},
                    },
                ],
            }
        ],
        "temperature": 0.1,
        "max_tokens": 300,
    }

    response = requests.post(
        "http://localhost:1234/v1/chat/completions",
        json=payload,
        headers={"Content-Type": "application/json"},
        timeout=1200,
    )
    print(response.json()["choices"][0]["message"]["content"])
except Exception as e:
    print(e)
