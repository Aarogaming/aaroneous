import os
import glob
from collections import Counter


def process_file(filepath):
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()

    if not lines:
        return

    # We want to remove long repeating sections.
    # A simple way is to keep track of the last few lines, and if a line has appeared more than 5 times in the document, and it's consecutive or alternating, we remove it.
    # Actually, a better heuristic: if a line appears more than 10 times in the whole file, just keep the first 3 occurrences, or simply keep a seen set for lines that are just formulas, or if it repeats consecutively.

    out_lines = []
    line_counts = Counter()

    for line in lines:
        clean_line = line.strip()
        if not clean_line:
            out_lines.append(line)
            continue

        line_counts[clean_line] += 1

        if line_counts[clean_line] > 5 and len(clean_line) > 3:
            # skip it if it's repeating excessively
            continue

        out_lines.append(line)

    if len(out_lines) < len(lines):
        print(f"Cleaned {filepath}: {len(lines)} -> {len(out_lines)} lines")
        with open(filepath, "w", encoding="utf-8") as f:
            f.writelines(out_lines)


if __name__ == "__main__":
    for file in glob.glob("**/*_extracted.txt", recursive=True):
        process_file(file)
