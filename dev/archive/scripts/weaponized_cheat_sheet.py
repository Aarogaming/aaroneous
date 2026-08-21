import os
import re


def abbreviate_subtext(text):
    # Standard engineering abbreviations to make sentences look like raw variable labels
    replacements = {
        r"\b[Ff]orward\b": "Fwd",
        r"\b[Rr]everse\b": "Rev",
        r"\b[Vv]oltage\b": "Volt",
        r"\b[Cc]urrent\b": "Curr",
        r"\b[Pp]ower\b": "Pwr",
        r"\b[Dd]issipation\b": "Diss",
        r"\b[Ss]ilicon\b": "Si",
        r"\b[Gg]ermanium\b": "Ge",
        r"\b[Dd]iode\b": "D",
        r"\b[Mm]aximum\b": "Max",
        r"\b[Mm]inimum\b": "Min",
        r"\b[Ee]quivalent\b": "Eq",
        r"\b[Ff]requency\b": "Freq",
        r"\b[Cc]apacitance\b": "Cap",
        r"\b[Ii]nductance\b": "Ind",
        r"\b[Rr]eactance\b": "React",
        r"\b[Ii]mpedance\b": "Imp",
        r"\b[Rr]esistance\b": "Res",
        r"\b[Rr]esistor\b": "Res",
        r"\b[Cc]ircuit\b": "Ckt",
        r"\b[Ss]ource\b": "Src",
        r"\b[Ii]nput\b": "In",
        r"\b[Oo]utput\b": "Out",
        r"\b[Pp]eak\b": "Pk",
        r"\b[Aa]verage\b": "Avg",
        r"\b[Zz]ener\b": "Znr",
    }

    # Strip grammar and stop-words
    for sw in [
        " a ",
        " an ",
        " the ",
        " is ",
        " to ",
        " for ",
        " of ",
        " and ",
        " in ",
        " approx ",
        " approximately ",
    ]:
        text = re.sub(sw, " ", text, flags=re.IGNORECASE)

    for pattern, rep in replacements.items():
        text = re.sub(pattern, rep, text)

    # Remove math symbols that crash MathJax
    text = text.replace("{", "(").replace("}", ")")
    text = text.replace("$", "").replace("_", " ").replace("\\", "")

    # Truncate to maximum 5-6 words so it doesn't look like a sentence
    words = [w for w in text.split() if w.strip()]
    if len(words) > 5:
        text = " ".join(words[:5])
    else:
        text = " ".join(words)

    return text.strip(":,.- ")


def generate_weaponized_formula_sheet(md_path, out_path):
    with open(md_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    weaponized_content = ""

    for line in lines:
        if line.strip().startswith("#"):
            continue

        match = re.search(r"\$(.*?)\$", line)
        if match:
            formula = match.group(1).strip()

            text_before = line[: match.start()].strip("*- ")
            text_after = line[match.end() :].strip(" \n")

            if text_after.startswith("(") and text_after.endswith(")"):
                text_after = text_after[1:-1].strip()

            combined_text = f"{text_before} {text_after}".strip()

            if combined_text:
                short_text = abbreviate_subtext(combined_text)
                weaponized_formula = (
                    f"{formula} \\quad \\text{{\\scriptsize [{short_text}]}}"
                )
            else:
                weaponized_formula = formula

            weaponized_content += (
                f'<div class="formula-item">\\( {weaponized_formula} \\)</div>\n'
            )

    # The HTML template
    html_template = """<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Weaponized Formula Sheet</title>
    <script>
        MathJax = {
            tex: {
                inlineMath: [['\\\\(', '\\\\)']],
                displayMath: [['$$', '$$'], ['\\\\[', '\\\\]']],
                processEscapes: true
            },
            chtml: { 
                scale: 0.82, 
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

    final_html = html_template.replace("__CONTENT__", weaponized_content)

    with open(out_path, "w", encoding="utf-8") as f:
        f.write(final_html)
    print(f"Generated WEAPONIZED formula sheet: {out_path}")


if __name__ == "__main__":
    base_dir = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"

    files_to_process = [
        (
            os.path.join(base_dir, "ETE 120", "ETE120_Formula_Sheet_3Q.md"),
            os.path.join(base_dir, "ETE 120", "ETE120_CheatSheet_Weaponized.html"),
        ),
        (
            os.path.join(base_dir, "ETM 110", "ETM110_Formula_Sheet_3Q.md"),
            os.path.join(base_dir, "ETM 110", "ETM110_CheatSheet_Weaponized.html"),
        ),
    ]

    for md_file, html_file in files_to_process:
        if os.path.exists(md_file):
            generate_weaponized_formula_sheet(md_file, html_file)
        else:
            print(f"Could not find markdown file: {md_file}")
