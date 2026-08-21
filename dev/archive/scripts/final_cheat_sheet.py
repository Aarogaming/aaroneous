import os
import re

ALLOWED_CONSTANTS = {
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    25,
    0.025,
    100,
    120,
    180,
    270,
    360,
    0.7,
    0.3,
    1.4,
    0.6,
    1.2,
    0.707,
    0.7071,
    1.414,
    0.636,
    0.318,
    1.732,
}


def is_generic_formula(formula):
    f_clean = formula.replace(" ", "").replace("\n", "")

    # 1. Needs to be an equation or inequality
    if not any(sym in f_clean for sym in ["=", "\\approx", "<", ">", "\\sim"]):
        return False

    # 2. Reject specific text keywords and long text blocks
    text_blocks = re.findall(r"\\text\{(.*?)\}", formula)
    for tb in text_blocks:
        words = tb.split()
        if len(words) > 3:
            return False

    lower_f = formula.lower()
    stop_words = [
        "measured",
        "calculated",
        "notes",
        "test",
        "lab",
        "example",
        "find",
        "solve",
    ]
    if any(sw in lower_f for sw in stop_words):
        return False

    # 3. Reject specific engineering prefixes indicating a calculated value
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

    # 4. Strip text for pure math checks
    clean_math = re.sub(r"\\text\{.*?\}", "", formula)
    clean_math = re.sub(r"\\[a-zA-Z]+", "", clean_math)

    variables = re.findall(r"[a-zA-Z]", clean_math)
    if len(variables) == 0:
        return False

    numbers = [float(n) for n in re.findall(r"\b\d+(?:\.\d+)?\b", clean_math)]
    for n in numbers:
        if n not in ALLOWED_CONSTANTS:
            return False

    # 5. REJECT INDEX/VARIABLE DEFINITIONS (e.g. f = \text{frequency})
    # We split by the relational operator and verify BOTH sides have mathematical substance
    relational_pattern = r"(=|\\approx|<|>|\\sim)"
    parts = re.split(relational_pattern, formula)

    # parts will look like ['f ', '=', ' \\text{frequency}']
    if len(parts) >= 3:
        # Check left and right sides (ignoring the operator in the middle)
        for i in range(0, len(parts), 2):
            part = parts[i]
            # Clean this specific side of the equation
            p_clean = re.sub(r"\\text\{.*?\}", "", part)
            p_clean = re.sub(r"\\[a-zA-Z]+", "", p_clean)
            p_vars = re.findall(r"[a-zA-Z]", p_clean)
            p_nums = re.findall(r"\b\d+(?:\.\d+)?\b", p_clean)

            # If one side of the equation has NO variables and NO numbers after removing text,
            # it means the side was purely an English definition/index item!
            if len(p_vars) == 0 and len(p_nums) == 0:
                return False

    return True


def generate_absolute_strict_formula_sheet(md_path, out_path):
    with open(md_path, "r", encoding="utf-8") as f:
        content = f.read()

    inline_formulas = re.findall(r"\$(.*?)\$", content)
    block_formulas = re.findall(r"\$\$(.*?)\$\$", content, re.DOTALL)

    all_formulas = inline_formulas + [b.strip() for b in block_formulas]

    strict_content = ""
    seen_formulas = set()

    for formula in all_formulas:
        if len(formula.strip()) < 2:
            continue

        f_norm = formula.replace(" ", "").replace("\n", "")

        if f_norm in seen_formulas:
            continue

        seen_formulas.add(f_norm)

        if not is_generic_formula(formula):
            continue

        strict_content += f'<div class="formula-item">\\( {formula.strip()} \\)</div>\n'

    html_template = """<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Strict Formula Sheet</title>
    <script>
        MathJax = {
            tex: {
                inlineMath: [['\\\\(', '\\\\)']],
                displayMath: [['$$', '$$'], ['\\\\[', '\\\\]']],
                processEscapes: true
            },
            chtml: { 
                scale: 0.90, 
                displayAlign: 'left',
                displayIndent: '0'
            }
        };
    </script>
    <script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
    <style>
        @page { 
            size: 8.5in 11in portrait; 
            margin: 0.25in; 
        }
        
        body {
            font-family: sans-serif;
            background-color: #fff;
            margin: 0 auto;
            padding: 0;
            width: 8in;
        }

        #content {
            column-count: 3;
            column-gap: 0.2in;
            column-rule: 1px dashed #ccc;
        }
        
        .formula-item {
            break-inside: avoid;
            page-break-inside: avoid;
            margin-bottom: 8px;
            padding-bottom: 4px;
            border-bottom: 1px solid #f4f4f4;
            display: block;
        }
        
        mjx-container { margin: 0 !important; padding: 0 !important; }
    </style>
</head>
<body>
    <div id="content">
        __CONTENT__
    </div>
    <script>
        MathJax.typesetPromise();
    </script>
</body>
</html>"""

    final_html = html_template.replace("__CONTENT__", strict_content)

    with open(out_path, "w", encoding="utf-8") as f:
        f.write(final_html)
    print(f"Generated NO-INDEX formula sheet: {out_path}")


if __name__ == "__main__":
    base_dir = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"

    files_to_process = [
        (
            os.path.join(base_dir, "ETE 120", "ETE 120_Master_Formula_Sheet.md"),
            os.path.join(base_dir, "ETE 120", "ETE120_CheatSheet_AbsoluteStrict.html"),
        ),
        (
            os.path.join(base_dir, "ETM 110", "ETM 110_Master_Formula_Sheet.md"),
            os.path.join(base_dir, "ETM 110", "ETM110_CheatSheet_AbsoluteStrict.html"),
        ),
    ]

    for md_file, html_file in files_to_process:
        if os.path.exists(md_file):
            generate_absolute_strict_formula_sheet(md_file, html_file)
        else:
            print(f"Could not find markdown file: {md_file}")
