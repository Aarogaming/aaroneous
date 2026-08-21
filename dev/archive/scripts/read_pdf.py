import sys
import fitz


def extract_text(pdf_path):
    try:
        doc = fitz.open(pdf_path)
        text = ""
        for page in doc:
            text += page.get_text() + "\n"
        print(text)
    except Exception as e:
        print(f"Error reading PDF: {e}")


if __name__ == "__main__":
    if len(sys.argv) > 1:
        extract_text(sys.argv[1])
