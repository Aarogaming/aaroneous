# Text encoding contract

Repository text uses **UTF-8 without a byte-order mark (BOM), LF line endings,
and a final LF for nonempty files**. Empty files are valid. Unicode content is
preserved; this policy does not require ASCII, Unicode normalization, indentation
changes, or removal of Markdown trailing spaces.

`.editorconfig` guides editors. `.gitattributes` declares known source and
configuration formats as text, requests LF checkouts, and explicitly excludes
binary assets. Unknown formats retain automatic text detection. Git attributes
do not validate UTF-8 and do not rewrite existing files merely by being added.
Git may show an existing file as modified because its newly normalized view
differs from the index, even when its raw bytes are unchanged. Do not stage
such apparent changes as part of this foundation-only change.
PowerShell scripts follow the same contract and should be run with PowerShell 7;
Windows PowerShell 5.1 can misread non-ASCII UTF-8 scripts without a BOM.

## Read-only scanner

Requires Python 3.10+ and Git. Run from any directory; the default repository is
the parent of this script's directory. The scanner never normalizes, stages,
renormalizes, or writes files, and never invokes Git clean/smudge filters.

```console
python scripts/check_text_encoding.py
python scripts/check_text_encoding.py --inventory
python scripts/check_text_encoding.py --dry-run --json
python scripts/check_text_encoding.py --source index --inventory --json
python scripts/check_text_encoding.py --source index .editorconfig .gitattributes docs/contributing/TEXT_ENCODING.md scripts/check_text_encoding.py
python scripts/check_text_encoding.py --root /path/to/repository --inventory
```

- Default mode validates bytes of working-tree files listed in the Git index,
  using current working-tree Git attributes. Untracked and ignored files are
  outside the scan; stage new files before checking them.
- `--source index` validates staged blob bytes and uses `git check-attr --cached`.
  This distinguishes committed/staged content from an older CRLF checkout.
  Git's normal local/global attribute precedence still applies in both modes.
- Optional positional paths are exact, case-sensitive repository-relative names
  with `/` separators, not globs or directory names. Unknown paths are errors.
  `--root` must be the repository root, not a subdirectory.
- `--inventory` and `--dry-run` are aliases: report the same findings as strict
  validation but do not fail for encoding violations. All modes are read-only.
- Exit codes: **0** = clean validation or completed inventory; **1** = strict
  validation found violations; **2** = scan failure or usage error. Inventory
  never suppresses missing-file, unreadable-file, or unmerged-index failures.
- `--json` emits `source`, `summary`, and a `files` array. Each file has `path`,
  `classification`, and `issues`; scan failures also have `error`. A failure
  before scanning emits a top-level `error`. JSON escapes unusual filenames;
  human output also quotes paths safely.

Issue names are `utf8-bom`, `utf16-or-utf32-bom`, `invalid-utf8`, `nul-byte`,
`crlf`, `bare-cr`, `missing-final-newline`, `conflicting-eol-attribute`, and
`conflicting-encoding-attribute`. Several can apply to one file.

## Classification and safety

Explicit `-text`/`binary` attributes take precedence; such files are reported as
`binary-attribute` and their bytes are not opened or validated. Explicit `text`
files are validated even if they contain NUL or non-UTF-8 bytes. For automatic
or unspecified text, recognized UTF BOMs are checked as text; otherwise a NUL
in the first 8,000 bytes produces `binary-detected`. Other bytes are validated
as UTF-8: decode failure alone is deliberately not a binary exemption. This is
a conservative scanner heuristic, not a complete reproduction of Git's binary
heuristic. Declare additional binary formats explicitly when needed.

Tracked symlinks and submodules are inventoried as `symlink` and `submodule`
without traversal. Regular tracked files replaced by symlinks, Windows reparse
points, directories, or special files fail working-tree validation. Missing
tracked text candidates fail too (explicit binary exclusions are not opened).
The scanner uses NUL-delimited Git paths and rejects
unmerged index entries. It is intended for a stable local checkout, not as a
security boundary against concurrent filesystem replacement. It does not scan
submodule contents or interpret text embedded inside binary containers.

## Migration workflow

1. Run the inventory against both the worktree and index. Keep their results
   distinct: a CRLF checkout does not imply the staged blob contains CRLF.
2. Review binary classifications and manually inspect invalid UTF-8 or UTF-16/32
   findings before choosing a conversion. Never guess a legacy encoding or
   remove NUL bytes blindly.
3. In a separately authorized migration, change only reviewed text files,
   preserving content and binary bytes. Inspect the resulting diff.
4. Run strict validation against the migrated scope, then the full index and
   worktree. Use the inventory to track any remaining migration work.

This foundation introduces no source migration or CI integration. Do not use
blanket `git add --renormalize .` as a substitute for reviewing the inventory.

### Initial migration inventory (2026-09-15)

With these four foundation files staged, the working-tree scan covered 1,716
tracked files: 1,708 text files and eight explicit binary assets. It found 587
text files with issues and no scan errors. The original staged content had the
same issue counts before this foundation was staged. These are a baseline,
not an allowlist; rerun the commands above for the current per-file inventory.

| Issue | Files |
| --- | ---: |
| Missing final newline | 555 |
| UTF-8 BOM | 42 |
| Bare carriage return | 6 |
| CRLF | 3 |
| Invalid UTF-8 | 2 |

Counts overlap. The two files requiring an encoding decision before conversion
are `docs/archive/16_FLAGSHIP_WORKFLOW_AND_BENCHMARK_METHODOLOGY.md` and
`docs/forensics/IMMUNE_SYSTEM_SUMMARY.md`. No migration was performed here.
