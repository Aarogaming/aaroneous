import re
with open("MASTER_ROADMAP.md", "r") as f: content = f.read()
head_part1 = """- [ ] **Dynamic plugin hot-swap**: implement the command-buffer ABI proof-of-concept in `crates/api`/`studio_hud` per RFC-0006 (docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md). The unauthenticated, unsandboxed `crates/hotload`/`crates/plugin_api` dynamic-DLL-loading scaffolding this line used to point at was dead code with no caller and has been removed; it predated and duplicated the unsound pattern RFC-0006 already documents removing from `studio_hud`.
- [ ] **Cratify v2 enforcement backlog** ([docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md) section 7.1), in order: `[package.metadata.cratify] profile` on every crate; profile-aware unsafe policy; hot-path markers on `kernel` scan-loop code (currently 4 functions, none in the hypervisor scan loop); panic-on-runtime-input rule; ambient clock rule; self-started thread/task rule; baseline-count ratchet mode; retire the five library `#[allow(ambient_authority)]` exemptions; profile dependency direction check; audit scope for `xtask/`/`sdk/`/`benches/`; documentation link gate."""
content = re.sub(r'<<<<<<< HEAD\n- \[ \] \*\*Dynamic plugin hot-swap.*?\n=======\n- \[ \] \*\*Dynamic plugin hot-swap: implement.*?\n>>>>>>> origin/main', head_part1, content, flags=re.DOTALL)
match = re.search(r'<<<<<<< HEAD\n(.*?)=======\n.*?>>>>>>> origin/main', content, flags=re.DOTALL)
if match:
    table = match.group(1)
    table = re.sub(r'\| (\d+\.\d+\.\d+) \|', r'| v\1 |', table)
    content = content[:match.start()] + table + content[match.end():]
with open("MASTER_ROADMAP.md", "w") as f: f.write(content)
