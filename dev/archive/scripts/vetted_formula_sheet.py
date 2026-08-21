import os
import re
from master_formulas import APPROVED_FORMULAS


def generate_vetted_formula_sheet(md_path, out_path):
    # Separate the main formulas from the foundational ones for pictorial typography
    # We will identify them by checking if they contain the specific foundational substrings.

    series_forms = [f for f in APPROVED_FORMULAS if "(series)" in f]
    parallel_forms = [f for f in APPROVED_FORMULAS if "(parallel)" in f]
    wye_forms = [f for f in APPROVED_FORMULAS if "(Y)" in f]
    delta_forms = [f for f in APPROVED_FORMULAS if "(\\Delta)" in f]

    # Collect all pictorial forms to exclude from main flow
    pictorial_forms = set(series_forms + parallel_forms + wye_forms + delta_forms)

    main_content = ""
    for item in APPROVED_FORMULAS:
        if item not in pictorial_forms:
            main_content += f'<div class="formula-item">\\( {item} \\)</div>\n'

    # Build the pictorial HTML with explicit text alignment and CSS transforms
    pictorial_html = f"""
    <div id="pictorial-section">
        <!-- SERIES SQUARE -->
        <div class="shape-container series-square">
            <div class="boxed-eq s-top">\\( {series_forms[0] if len(series_forms) > 0 else ""} \\)</div>
            <div class="boxed-eq s-left">\\( {series_forms[1] if len(series_forms) > 1 else ""} \\)</div>
            <div class="boxed-eq s-bottom">\\( {series_forms[2] if len(series_forms) > 2 else ""} \\)</div>
            <div class="boxed-eq s-right">\\( P_{{T(series)}} = P_1 + P_2 + \\dots + P_n \\)</div>
        </div>
        
        <!-- PARALLEL LINES -->
        <div class="shape-container parallel-lines">
            <div class="p-col">
                <div class="boxed-eq p-item">\\( {parallel_forms[0] if len(parallel_forms) > 0 else ""} \\)</div>
                <div class="boxed-eq p-item">\\( {parallel_forms[2] if len(parallel_forms) > 2 else ""} \\)</div>
            </div>
            <div class="p-col">
                <div class="boxed-eq p-item">\\( {parallel_forms[1] if len(parallel_forms) > 1 else ""} \\)</div>
                <div class="boxed-eq p-item">\\( {parallel_forms[3] if len(parallel_forms) > 3 else ""} \\)</div>
            </div>
        </div>
        
        <div class="wye-delta-column">
            <!-- WYE (Y) -->
            <div class="shape-container wye-shape">
                <div class="boxed-eq y-left">\\( {wye_forms[0] if len(wye_forms) > 0 else ""} \\)</div>
                <div class="boxed-eq y-right">\\( {wye_forms[1] if len(wye_forms) > 1 else ""} \\)</div>
                <div class="boxed-eq y-bottom">\\( P_{{T(Y)}} = \\sqrt{{3}} \\cdot V_L \\cdot I_L \\cdot \\cos(\\theta) \\)</div>
            </div>
            
            <!-- DELTA (Triangle) -->
            <div class="shape-container delta-shape">
                <div class="boxed-eq d-left">\\( {delta_forms[0] if len(delta_forms) > 0 else ""} \\)</div>
                <div class="boxed-eq d-right">\\( {delta_forms[1] if len(delta_forms) > 1 else ""} \\)</div>
                <div class="boxed-eq d-bottom">\\( P_{{T(\\Delta)}} = \\sqrt{{3}} \\cdot V_L \\cdot I_L \\cdot \\cos(\\theta) \\)</div>
            </div>
        </div>
    </div>
    """

    html_template = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Formula Sheet</title>
    <script>
        MathJax = {{
            tex: {{
                inlineMath: [['\\\\(', '\\\\)']],
                displayMath: [['$$', '$$'], ['\\\\[', '\\\\]']],
                processEscapes: true
            }},
            chtml: {{ 
                scale: 0.90, 
                displayAlign: 'left',
                displayIndent: '0'
            }}
        }};
    </script>
    <script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
    <style>
        @page {{ 
            size: 8.5in 11in portrait; 
            margin: 0.25in; 
        }}
        
        body {{
            font-family: sans-serif;
            background-color: #fff;
            margin: 0 auto;
            padding: 0;
            width: 8in;
        }}

        #main-content {{
            column-count: 3;
            column-gap: 0.2in;
            column-rule: 1px dashed #ccc;
            margin-bottom: 20px;
        }}
        
        .formula-item {{
            break-inside: avoid;
            page-break-inside: avoid;
            margin-bottom: 8px;
            padding-bottom: 4px;
            border-bottom: 1px solid #f4f4f4;
            display: block;
        }}
        
        mjx-container {{ margin: 0 !important; padding: 0 !important; }}
        
        /* PICTORIAL TYPOGRAPHY SECTION */
        #pictorial-section {{
            display: flex;
            justify-content: space-around;
            align-items: center;
            margin-top: 20px;
            padding-top: 20px;
            border-top: 2px solid #000;
            page-break-inside: avoid;
            width: 100%;
            box-sizing: border-box;
        }}
        
        .shape-container {{
            position: relative;
            background-color: transparent;
        }}

        /* Strict sizing to fit 3 columns on an 8.5in page (max body width ~768px) */
        /* 240px * 3 = 720px, leaves safe margins */
        .series-square {{ width: 240px; height: 260px; }}
        .parallel-lines {{ width: 240px; height: 260px; display: flex; justify-content: space-between; padding: 0 10px; align-items: center; box-sizing: border-box; }}
        
        .wye-delta-column {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: flex-start;
            gap: 30px;
            width: 240px;
        }}
        .wye-shape {{ width: 240px; height: 220px; }}
        .delta-shape {{ width: 240px; height: 220px; }}
        
        .boxed-eq {{
            border: 2px solid #333;
            padding: 6px 10px;
            background-color: #fff;
            white-space: nowrap;
            display: inline-block;
            box-shadow: 2px 2px 0px rgba(0,0,0,0.15);
            z-index: 2;
            font-size: 0.9em; /* slightly smaller to guarantee no overlap */
        }}
        
        /* Series Square - Using explicit center-point coordinates */
        .series-square .s-top {{ position: absolute; top: 20px; left: 120px; transform: translate(-50%, -50%); }}
        .series-square .s-bottom {{ position: absolute; top: 240px; left: 120px; transform: translate(-50%, -50%); }}
        .series-square .s-left {{ position: absolute; top: 130px; left: 20px; transform: translate(-50%, -50%) rotate(-90deg); }}
        .series-square .s-right {{ position: absolute; top: 130px; left: 220px; transform: translate(-50%, -50%) rotate(90deg); }}
        
        /* Parallel Lines */
        .parallel-lines .p-col {{ 
            display: flex; 
            flex-direction: column; 
            justify-content: space-evenly;
            height: 100%;
            gap: 20px;
        }}
        
        /* Wye (Y) - Explicit absolute centers */
        .wye-shape .y-left {{ position: absolute; top: 50px; left: 50px; transform: translate(-50%, -50%) rotate(-30deg); }}
        .wye-shape .y-right {{ position: absolute; top: 50px; left: 190px; transform: translate(-50%, -50%) rotate(30deg); }}
        .wye-shape .y-bottom {{ position: absolute; top: 160px; left: 120px; transform: translate(-50%, -50%) rotate(90deg); }}
        
        /* Delta (Triangle) - Explicit absolute centers for equilateral corners */
        .delta-shape .d-left {{ position: absolute; top: 120px; left: 80px; transform: translate(-50%, -50%) rotate(60deg); }}
        .delta-shape .d-right {{ position: absolute; top: 120px; left: 160px; transform: translate(-50%, -50%) rotate(-60deg); }}
        .delta-shape .d-bottom {{ position: absolute; top: 200px; left: 120px; transform: translate(-50%, -50%); }}
        
    </style>
</head>
<body>
    <div id="main-content">
        {main_content}
    </div>
    {pictorial_html}
    <script>
        MathJax.typesetPromise();
    </script>
</body>
</html>"""

    with open(out_path, "w", encoding="utf-8") as f:
        f.write(html_template)

    print(f"Generated PERFECT formula sheet with PICTORIAL TYPOGRAPHY: {out_path}")


if __name__ == "__main__":
    base_dir = r"C:\\Users\\aarog\\OneDrive - St. Clair County Community College\\Documents\\College"

    files_to_process = [
        (
            os.path.join(base_dir, "ETE 120", "ETE 120_Master_Formula_Sheet.md"),
            os.path.join(base_dir, "ETE 120", "ETE120_FormulaSheet_Vetted.html"),
        ),
        (
            os.path.join(base_dir, "ETM 110", "ETM 110_Master_Formula_Sheet.md"),
            os.path.join(base_dir, "ETM 110", "ETM110_FormulaSheet_Vetted.html"),
        ),
    ]

    for md_file, html_file in files_to_process:
        # We don't actually need the markdown file anymore since we rely purely on the master list
        generate_vetted_formula_sheet(md_file, html_file)
