use rkyv::{Archive, Serialize, Deserialize};
use serde::{Serialize as SerdeSerialize, Deserialize as SerdeDeserialize};

/// Core binary IR used by the inner & outer loops.
#[derive(Debug, Archive, Serialize, Deserialize, SerdeSerialize, SerdeDeserialize)]
#[archive(check_bytes)]
pub enum Instruction {
    /// Mutate an AST node identified by a file ID and a patch signature.
    MutateAST {
        file_id: u64,
        patch_sig: Vec<u8>,
    },
    /// Execute a WASM sandbox with the given binary payload.
    RunWasm {
        wasm_bytes: Vec<u8>,
    },
    /// Compile and verify a generated Rust crate (path to crate root).
    CompileCrate {
        crate_path: String,
    },
    /// No-op placeholder for future instructions.
    NoOp,
}

/// Encode an Instruction to a zero-copy byte vector.
pub fn encode(instr: &Instruction) -> Result<Vec<u8>, anyhow::Error> {
    // Serialize the instruction; this should never fail under normal conditions.
    let archived = rkyv::to_bytes::<_, 256>(instr).map_err(|e| anyhow::anyhow!("serialization failed: {}", e))?;
    Ok(archived.to_vec())
}

/// Decode a byte slice back into an Instruction.
pub fn decode(buf: &[u8]) -> Result<Instruction, rkyv::error::Error> {
    let archived = unsafe { rkyv::archived_root::<Instruction>(buf) };
    archived.deserialize(&mut rkyv::de::deserializers::SharedDeserializeMap::default())
}
