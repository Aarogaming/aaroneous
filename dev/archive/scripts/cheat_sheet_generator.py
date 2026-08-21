import os


def generate_cheat_sheet(md_path, out_path):
    with open(md_path, "r", encoding="utf-8") as f:
        md_content = f.read()

    # The HTML template uses extreme CSS density to fit everything onto one double-sided page.
    # It utilizes CSS Multi-column layout, tiny fonts, and removes massive headers.
    html_template = """<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Cheat Sheet - US Letter</title>
    <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
    <script>
        MathJax = {
            tex: {
                inlineMath: [['$', '$'], ['\\\\(', '\\\\)']],
                displayMath: [['$$', '$$'], ['\\\\[', '\\\\]']],
                processEscapes: true
            },
            chtml: { 
                scale: 0.82, /* Optimal scale for US Letter 3-column */
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
            line-height: 1.1;
            color: #000;
            margin: 0 auto;
            padding: 0;
            width: 8in; /* 8.5in minus 0.25in margins on each side */
            background-color: #fff;
        }

        #content {
            column-count: 3;
            column-gap: 0.25in;
            column-rule: 1px solid #ccc;
        }
        
        /* Remove massive titles and space-wasting headers */
        h1 { display: none; }
        h2, h3, h4, h5, h6 {
            font-size: 9pt;
            margin: 6px 0 2px 0;
            padding: 0;
            text-transform: uppercase;
            border-bottom: 1px solid #000;
            break-after: avoid; 
            page-break-after: avoid;
        }
        
        p { margin: 2px 0; }
        ul, ol { margin: 2px 0; padding-left: 12px; }
        li { margin-bottom: 1px; }
        
        /* Math block optimizations */
        mjx-container { margin: 2px 0 !important; }
        
        /* Create physical boundary rules to keep formulas and text together */
        .rule-block {
            break-inside: avoid;
            page-break-inside: avoid;
            margin-bottom: 8px;
            padding-bottom: 4px;
            border-bottom: 1px dashed #eee;
        }
    </style>
</head>
<body>
    <div id="content"></div>
    <textarea id="markdown-source" style="display:none;">__CONTENT__</textarea>
    <script>
        let html = marked.parse(document.getElementById('markdown-source').value);
        html = html.replace(/(<h[2-6]>.*?)(?=<h[2-6]>|$)/gs, '<div class="rule-block">$1</div>');
        document.getElementById('content').innerHTML = html;
        MathJax.typesetPromise();
    </script>
</body>
</html>"""

    final_html = html_template.replace("__CONTENT__", md_content)

    with open(out_path, "w", encoding="utf-8") as f:
        f.write(final_html)
    print(f"Generated professional cheat sheet: {out_path}")


if __name__ == "__main__":
    base_dir = r"C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"

    # We apply this algorithm to the 3Q Formula Sheets we generated earlier tonight
    files_to_process = [
        (
            os.path.join(base_dir, "ETE 120", "ETE120_Formula_Sheet_3Q.md"),
            os.path.join(base_dir, "ETE 120", "ETE120_CheatSheet_Printable.html"),
        ),
        (
            os.path.join(base_dir, "ETM 110", "ETM110_Formula_Sheet_3Q.md"),
            os.path.join(base_dir, "ETM 110", "ETM110_CheatSheet_Printable.html"),
        ),
    ]

    for md_file, html_file in files_to_process:
        if os.path.exists(md_file):
            generate_cheat_sheet(md_file, html_file)
        else:
            print(f"Could not find markdown file: {md_file}")
