// src/scaffold.rs
use anyhow::Result;
use crate::generator;
use crate::workspace;


pub async fn run(name: &str) -> Result<()> {
    // Generate crate skeleton
    generator::generate_crate(name)?;
    // Find workspace root
    let cwd = std::env::current_dir()?;
    let root = workspace::find_workspace_root(&cwd)?;
    // Add the new crate as a workspace member
    let crate_path = root.join("crates").join(name);
    workspace::add_workspace_member(&root, &crate_path)?;
    Ok(())
}
