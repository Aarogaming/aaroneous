import re
import os


def is_generic_formula(formula):
    f_clean = formula.replace(" ", "").replace("\n", "")

    # 1. Needs to be an equation or inequality
    if not any(sym in f_clean for sym in ["=", "\\approx", "<", ">", "\\sim"]):
        return False

    # 2. Reject specific text keywords that indicate lab notes or specific answers
    lower_f = formula.lower()
    stop_words = ["measured", "calculated", "notes", "test", "lab", "example"]
    if any(sw in lower_f for sw in stop_words):
        return False

    # 3. Reject specific engineering prefixes indicating a calculated value
    # Formulas rarely use 'mA', 'k\Omega', 'uF', 'pF', 'mH' explicitly
    prefixes = [
        r"m\text{A}",
        r"k\\Omega",
        r"\\mu\text{F}",
        r"p\text{F}",
        r"m\text{H}",
        r"k\text{Hz}",
        r"M\text{Hz}",
    ]
    if any(re.search(p, formula, re.IGNORECASE) for p in prefixes):
        return False

    # 4. Remove text blocks and standard latex commands to count numbers/variables
    clean = re.sub(r"\\text\{.*?\}", "", formula)
    clean = re.sub(r"\\[a-zA-Z]+", "", clean)

    numbers = [float(n) for n in re.findall(r"\b\d+(?:\.\d+)?\b", clean)]
    variables = re.findall(r"[a-zA-Z]", clean)

    if len(variables) == 0:
        return False

    if len(numbers) > 3:
        return False

    # Reject large numbers except common angles/constants
    for n in numbers:
        if n > 15 and n not in [100, 120, 180, 270, 360]:
            return False

    # 5. Check for assignments of a variable to a specific arbitrary float (e.g. V = 1.414V)
    # Split by '=' and check if one side is just a float
    parts = formula.split("=")
    if len(parts) == 2:
        left, right = parts[0], parts[1]

        # Check right side
        r_clean = re.sub(r"\\text\{.*?\}", "", right)
        r_clean = re.sub(r"\\[a-zA-Z]+", "", r_clean)
        r_vars = re.findall(r"[a-zA-Z]", r_clean)
        r_nums = re.findall(r"\b\d+(?:\.\d+)?\b", r_clean)

        # If the right side has numbers but NO variables (ignoring units inside \text{})
        if len(r_vars) == 0 and len(r_nums) > 0:
            # We allow 0.7, 0.3, 1.4 (diode constants), 0 (ground), 1, -1
            if not any(n in ["0.7", "0.3", "1.4", "0", "1", "-1"] for n in r_nums):
                return False

    return True


with open("ETE 120/ETE 120_Master_Formula_Sheet.md", "r", encoding="utf-8") as f:
    content = f.read()

formulas = re.findall(r"\$\$(.*?)\$\$", content, re.DOTALL)
formulas = [f.strip() for f in formulas if f.strip()]

accepted = []
rejected = []
seen = set()

for f in formulas:
    f_norm = f.replace(" ", "").replace("\n", "")
    if f_norm in seen:
        continue
    seen.add(f_norm)

    if is_generic_formula(f):
        accepted.append(f)
    else:
        rejected.append(f)

print(f"Total Unique Math Blocks: {len(seen)}")
print(f"Accepted Formulas: {len(accepted)}")
print(f"Rejected Calculations: {len(rejected)}")
print("\n--- SAMPLE ACCEPTED ---")
for f in accepted[:10]:
    print(f)
print("\n--- SAMPLE REJECTED ---")
for f in rejected[:10]:
    print(f)
