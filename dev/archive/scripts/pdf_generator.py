import os
import matplotlib.pyplot as plt
import matplotlib.patches as patches
from master_formulas import APPROVED_FORMULAS


def generate_pdf(course_name, out_path):
    # Separate formulas
    series_forms = [f for f in APPROVED_FORMULAS if "(series)" in f]
    parallel_forms = [f for f in APPROVED_FORMULAS if "(parallel)" in f]
    wye_forms = [f for f in APPROVED_FORMULAS if "(Y)" in f]
    delta_forms = [f for f in APPROVED_FORMULAS if "(\\Delta)" in f]

    pictorial_forms = set(series_forms + parallel_forms + wye_forms + delta_forms)
    main_forms = [f for f in APPROVED_FORMULAS if f not in pictorial_forms]

    # Create an 8.5 x 11 inch figure
    fig = plt.figure(figsize=(8.5, 11))

    # We use figure coordinates (0 to 1) for precise placement
    # 0,0 is bottom left. 1,1 is top right.

    # --- 1. MAIN FORMULAS (Columns) ---
    col_x = [0.1, 0.4, 0.7]
    y_start = 0.95
    y_step = 0.025

    col_idx = 0
    current_y = y_start

    for f in main_forms:
        # matplotlib math text doesn't like \le and some others
        f = f.replace(r"\le ", r"\leq ")  # add space so \left doesn't break
        f = f.replace(r"\dots", "...")
        fig.text(
            col_x[col_idx], current_y, f"${f}$", fontsize=10, ha="left", va="center"
        )
        current_y -= y_step
        if current_y < 0.4:  # Leave bottom 40% of the page for shapes
            current_y = y_start
            col_idx += 1
            if col_idx > 2:
                break  # Out of space in this simple layout

    # --- 2. PICTORIAL TYPOGRAPHY (Bottom 40%) ---
    # We'll define centers for our shapes in figure coordinates (0 to 1)

    # Box drawing helper
    def draw_eq(x, y, text, rot=0):
        # We add a small bbox to mimic the "boxed-eq" look
        text = text.replace(r"\le ", r"\leq ")
        text = text.replace(r"\dots", r"...")
        fig.text(
            x,
            y,
            f"${text}$",
            fontsize=9,
            ha="center",
            va="center",
            rotation=rot,
            bbox=dict(
                facecolor="white", edgecolor="black", boxstyle="round,pad=0.3", alpha=1
            ),
        )

    # A. SERIES SQUARE (Bottom Left)
    cx, cy = 0.2, 0.2
    r = 0.08
    if len(series_forms) > 0:
        draw_eq(cx, cy + r, series_forms[0], 0)
    if len(series_forms) > 2:
        draw_eq(cx, cy - r, series_forms[2], 0)
    if len(series_forms) > 1:
        draw_eq(cx - r * 0.8, cy, series_forms[1], 90)
    draw_eq(cx + r * 0.8, cy, "P_{T(series)} = P_1 + P_2 + \dots + P_n", 270)

    # B. PARALLEL COLUMNS (Bottom Center-Left)
    cx, cy = 0.45, 0.2
    dx, dy = 0.05, 0.05
    if len(parallel_forms) > 0:
        draw_eq(cx - dx, cy + dy, parallel_forms[0], 0)
    if len(parallel_forms) > 2:
        draw_eq(cx - dx, cy - dy, parallel_forms[2], 0)
    if len(parallel_forms) > 1:
        draw_eq(cx + dx, cy + dy, parallel_forms[1], 0)
    if len(parallel_forms) > 3:
        draw_eq(cx + dx, cy - dy, parallel_forms[3], 0)

    # C. WYE OVER DELTA (Bottom Right)
    # Wye (Top)
    cx, cy = 0.75, 0.28
    if len(wye_forms) > 0:
        draw_eq(cx - 0.04, cy + 0.04, wye_forms[0], 30)
    if len(wye_forms) > 1:
        draw_eq(cx + 0.04, cy + 0.04, wye_forms[1], -30)
    draw_eq(
        cx, cy - 0.04, "P_{T(Y)} = \sqrt{3} \cdot V_L \cdot I_L \cdot \cos(\\theta)", 90
    )

    # Delta (Bottom)
    cx, cy = 0.75, 0.12
    if len(delta_forms) > 0:
        draw_eq(cx - 0.04, cy + 0.02, delta_forms[0], 60)
    if len(delta_forms) > 1:
        draw_eq(cx + 0.04, cy + 0.02, delta_forms[1], -60)
    draw_eq(
        cx,
        cy - 0.04,
        "P_{T(\\Delta)} = \sqrt{3} \cdot V_L \cdot I_L \cdot \cos(\\theta)",
        0,
    )

    # Save out
    plt.savefig(out_path, format="pdf", dpi=300, bbox_inches="tight")
    plt.close(fig)
    print(f"Generated Vector PDF: {out_path}")


if __name__ == "__main__":
    base_dir = r"C:\\Users\\aarog\\OneDrive - St. Clair County Community College\\Documents\\College"

    generate_pdf(
        "ETE 120", os.path.join(base_dir, "ETE 120", "ETE120_FormulaSheet_Vector.pdf")
    )
    generate_pdf(
        "ETM 110", os.path.join(base_dir, "ETM 110", "ETM110_FormulaSheet_Vector.pdf")
    )
