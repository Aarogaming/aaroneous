import os
from master_formulas import ETE_120_FORMULAS, ETM_110_FORMULAS


def generate_tex(course_name, out_path):
    if course_name == "ETE 120":
        APPROVED_FORMULAS = ETE_120_FORMULAS
    else:
        APPROVED_FORMULAS = ETM_110_FORMULAS
    # Categorize formulas
    series_forms = [f for f in APPROVED_FORMULAS if "(series)" in f]
    parallel_forms = [f for f in APPROVED_FORMULAS if "(parallel)" in f]
    wye_forms = [f for f in APPROVED_FORMULAS if "(Y)" in f]
    delta_forms = [f for f in APPROVED_FORMULAS if "(\\Delta)" in f]

    pictorial_forms = set(series_forms + parallel_forms + wye_forms + delta_forms)
    main_forms = [f for f in APPROVED_FORMULAS if f not in pictorial_forms]

    # Helper to safely get formula or empty string
    def get_form(lst, idx, default=""):
        return lst[idx] if len(lst) > idx else default

    # PAGE 1: Main Formulas
    main_tex = ""
    for f in main_forms:
        main_tex += f"\\noindent $\\displaystyle {f}$ \\\\[0.4cm]\n"

    # Image for PAGE 2
    # Ensure graphicx is imported and image is placed instead of tikz
    img_path = "../PXL_20260407_203941862.jpg"

    tex_document = f"""\\documentclass[10pt, letterpaper]{{article}}
\\usepackage[margin=0.4in]{{geometry}}
\\usepackage{{amsmath}}
\\usepackage{{amssymb}}
\\usepackage{{graphicx}}
\\usepackage{{multicol}}

\\pagestyle{{empty}}

\\begin{{document}}

% --- PAGE 1: CHRONOLOGICAL MAIN FORMULAS ---
\\begin{{multicols*}}{{3}}
{main_tex}
\\end{{multicols*}}

\\newpage
% --- PAGE 2: PICTORIAL TYPOGRAPHY (IMAGE OVERLAY) ---
\\begin{{center}}
\\includegraphics[width=\\textwidth,height=0.9\\textheight,keepaspectratio]{{{img_path}}}
\\end{{center}}

\\end{{document}}
"""

    with open(out_path, "w", encoding="utf-8") as f:
        f.write(tex_document)
    print(f"Generated LaTeX source: {out_path}")


if __name__ == "__main__":
    base_dir = r"C:\\Users\\aarog\\OneDrive - St. Clair County Community College\\Documents\\College"
    generate_tex(
        "ETE 120", os.path.join(base_dir, "ETE 120", "ETE120_FormulaSheet_Perfect.tex")
    )
    generate_tex(
        "ETM 110", os.path.join(base_dir, "ETM 110", "ETM110_FormulaSheet_Perfect.tex")
    )
