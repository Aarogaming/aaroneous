import os
import re


def create_strict_formula_sheet(md_path, out_path):
    with open(md_path, "r", encoding="utf-8") as f:
        lines = f.readlines()

    strict_content = ""
    for line in lines:
        # Rule 1: NO headers or titles
        if line.strip().startswith("#"):
            continue

        # Rule 2: If the line contains a math formula (indicated by $), keep it
        # This keeps the formula and its inline definition/subtext
        if "$" in line:
            # Clean up markdown bullet points for a cleaner look
            clean_line = re.sub(r"^\s*\*\s*", "", line).strip()
            clean_line = re.sub(r"^\s*-\s*", "", clean_line).strip()

            # Format the output line
            strict_content += f'<div class="formula-item">{clean_line}</div>\n'

    # The HTML template with extreme CSS density and no headers
    html_template = """<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Strict Formula Sheet</title>
    <script>
        MathJax = {
            tex: {
                inlineMath: [['$', '$'], ['\\\\(', '\\\\)']],
                displayMath: [['$$', '$$'], ['\\\\[', '\\\\]']],
                processEscapes: true
            },
            chtml: { 
                scale: 0.85, 
                displayAlign: 'left',
                displayIndent: '0'
            }
        };
    </script>
    <script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
    <style>
        /* Force strict US Letter dimensions for standard printer paper */
        @page { 
            size: 8.5in 11in portrait; 
            margin: 0.25in; 
        }
        
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            font-size: 8pt; 
            line-height: 1.2;
            color: #000;
            margin: 0 auto;
            padding: 0;
            width: 8in;
            background-color: #fff;
        }

        #content {
            column-count: 3;
            column-gap: 0.25in;
            column-rule: 1px dashed #ccc;
        }
        
        /* Math block optimizations */
        mjx-container { margin: 1px 0 !important; }
        
        /* Strict item formatting */
        .formula-item {
            break-inside: avoid;
            page-break-inside: avoid;
            margin-bottom: 6px;
            padding-bottom: 3px;
            border-bottom: 1px solid #f0f0f0;
            display: block;
        }
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
    print(f"Generated STRICT formula sheet: {out_path}")


if __name__ == "__main__":
    base_dir = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"

    files_to_process = [
        (
            os.path.join(base_dir, "ETE 120", "ETE120_Formula_Sheet_3Q.md"),
            os.path.join(base_dir, "ETE 120", "ETE120_CheatSheet_Strict.html"),
        ),
        (
            os.path.join(base_dir, "ETM 110", "ETM110_Formula_Sheet_3Q.md"),
            os.path.join(base_dir, "ETM 110", "ETM110_CheatSheet_Strict.html"),
        ),
    ]

    for md_file, html_file in files_to_process:
        if os.path.exists(md_file):
            create_strict_formula_sheet(md_file, html_file)
        else:
            print(f"Could not find markdown file: {md_file}")
