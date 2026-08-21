#!/usr/bin/env python3
"""
deconstruction_triage.py
Aaroneous Morphogenesis & Deconstruction Triage Tool:
Scans the entire repository and sorts every directory/file into the 5 Sovereign Destination Buckets,
identifying ghost directories, dead artifacts, and core organs for clean reconstruction.
"""

import os
import sys
import json

def triage_item(rel_path, is_dir, file_count):
    p = rel_path.lower().replace("\\", "/")

    # 1. ARCHIVE & PURGE
    if is_dir and file_count == 0 and "agents/" in p:
        return "PURGE_GHOST_DIRECTORY", "Empty agent shell directory containing only build residue"
    if "wasm" in p or "universal_sab" in p:
        return "ARCHIVE_WASM", "Deprecated WebAssembly / WIT artifact"
    if "scripts/" in p and ("cheat_sheet" in p or "extract" in p or "formula" in p or "philosopher" in p):
        return "ARCHIVE_SCRIPT", "Ad-hoc OCR/PDF/formula extraction script"
    if "docs/" in p and not "dev/" in p and any(k in p for k in ["cleanup", "migration", "final_", "task_complete", "restructuring"]):
        return "ARCHIVE_LEGACY_DOC", "Conflicting legacy summary document"

    # 2. ORGANS (Core Programs)
    if "marionette" in p or "win32_intercept" in p or "spatial_kinetic" in p:
        return "ORGAN_MARIONETTE", "User emulation, visual perception, or probing engine"
    if "chimera" in p or "deconstruction" in p or "scientific_analyzer" in p:
        return "ORGAN_CHIMERA", "Software adaptation, AST parsing, or decompilation engine"
    if "constellation" in p or "omni" in p or "spectral_layout" in p:
        return "ORGAN_OMNI", "3D Galaxy semantic navigation and star-node search"
    if "hypervisor" in p or "nervous_system" in p or "autonomic_loop" in p:
        return "ORGAN_AARONEOUS_MASTER", "Master supervisor, shared memory synapse, and metabolic governor"
    if "specialist" in p or "odin" in p or "merlin" in p or "ariel" in p:
        return "ORGAN_SPECIALISTS", "Sovereign specialist agent"
    if "maelstromui" in p:
        return "ORGAN_ARIEL_UI", "Tauri/React frontend interface"

    # 3. LIBRARIES
    if "components/biology" in p:
        return "LIBRARY_BIOLOGY", "Thermodynamic and metabolic governor library"
    if "components/compute" in p or "symbolic_math" in p or "predictive_models" in p:
        return "LIBRARY_COMPUTE", "Pure math, Markov, Bayesian, and Kalman filter library"
    if "components/paths" in p:
        return "LIBRARY_PATHS", "Workspace path resolver"
    if "components/control" in p or "components/hive" in p or "components/agents" in p:
        return "LIBRARY_AGENT_FRAMEWORK", "Shared agent traits, state tracking, and control messages"
    if "components/storage" in p or "persistence" in p:
        return "LIBRARY_STORAGE", "Database and disk persistence abstractions"

    # 4. GENETICS & DATA
    if "genetics" in p or "models" in p or "hox" in p or ".db" in p or "shaders" in p:
        return "DATA_AND_GENETICS", "Neural weights, LoRA adapters, HOX databases, or GPU shaders"

    # 5. DEV & MAINTENANCE
    if "dev/" in p:
        return "DEV_HOMEBASE", "Authoritative developer documentation, forensics, and maintenance tooling"

    # Default
    return "UNCATEGORIZED", "General utility or configuration"

def main():
    workspace_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
    print("=" * 60)
    print("  Aaroneous Deconstruction & Triage Scanner")
    print(f"  Scanning: {workspace_root}")
    print("=" * 60)

    triage_results = {
        "summary": {},
        "ghost_directories": [],
        "purge_candidates": [],
        "organs": [],
        "libraries": [],
        "data_and_genetics": [],
        "dev_homebase": [],
        "all_items": []
    }

    ignored_walk = {"target", "node_modules", ".git", "cache", "dist", "fabrication"}

    for root, dirs, files in os.walk(workspace_root):
        dirs[:] = [d for d in dirs if d not in ignored_walk]
        rel_dir = os.path.relpath(root, workspace_root)

        # Check directory itself
        if rel_dir != ".":
            bucket, reason = triage_item(rel_dir, True, len(files))
            if bucket == "PURGE_GHOST_DIRECTORY":
                triage_results["ghost_directories"].append({
                    "path": rel_dir.replace("\\", "/"),
                    "reason": reason
                })

        for f in files:
            full_path = os.path.join(root, f)
            rel_file = os.path.relpath(full_path, workspace_root).replace("\\", "/")
            bucket, reason = triage_item(rel_file, False, 1)

            entry = {
                "file": rel_file,
                "bucket": bucket,
                "reason": reason
            }

            triage_results["all_items"].append(entry)
            triage_results["summary"][bucket] = triage_results["summary"].get(bucket, 0) + 1

            if "ORGAN" in bucket:
                triage_results["organs"].append(entry)
            elif "LIBRARY" in bucket:
                triage_results["libraries"].append(entry)
            elif "PURGE" in bucket or "ARCHIVE" in bucket:
                triage_results["purge_candidates"].append(entry)
            elif "DATA" in bucket:
                triage_results["data_and_genetics"].append(entry)
            elif "DEV" in bucket:
                triage_results["dev_homebase"].append(entry)

    out_file = os.path.join(workspace_root, "dev", "tools", "maintenance", "triage_ledger.json")
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(triage_results, f, indent=2)

    print(f"\n[+] Triage Complete! Results written to: {out_file}")
    print("\n--- Bucket Distribution ---")
    for b, count in sorted(triage_results["summary"].items()):
        print(f"  {b:30}: {count} items")

    if triage_results["ghost_directories"]:
        print(f"\n[!] Discovered {len(triage_results['ghost_directories'])} Ghost Shell Directories:")
        for gd in triage_results["ghost_directories"]:
            print(f"    - {gd['path']}")

if __name__ == "__main__":
    main()
