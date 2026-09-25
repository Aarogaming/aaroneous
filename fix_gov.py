import re
with open("crates/governance/Cargo.toml", "r") as f: content = f.read()
replacement = """z3 = { version = "0.21", optional = true }\nrand = { version = "0.8", optional = true }"""
content = re.sub(r'<<<<<<< HEAD\nz3 = { version = "0\.9", optional = true }\nrand = { version = "0\.8", optional = true }\n=======\nz3 = { version = "0\.21", optional = true }\n>>>>>>> origin/main', replacement, content, flags=re.DOTALL)
with open("crates/governance/Cargo.toml", "w") as f: f.write(content)
