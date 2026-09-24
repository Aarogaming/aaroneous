import re
with open("crates/compute/src/si_solid_state.rs", "r") as f: content = f.read()

replacement = """        // SAFETY: `file` is a fresh handle this call just opened; `mmap2`'s
        // precondition is that it isn't concurrently modified, and `mmap`
        // is only read from for the rest of this function.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };"""

content = re.sub(r'<<<<<<< HEAD\n        let mmap = unsafe \{ memmap2::Mmap::map\(&file\)\? \};\n=======\n.*?>>>>>>> origin/main', replacement, content, flags=re.DOTALL)
with open("crates/compute/src/si_solid_state.rs", "w") as f: f.write(content)
