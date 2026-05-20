// AAS-IR (Intermediate Representation)
// Standardized binary instruction set for Aaroneous WASM Enzymes.
// Derived from legacy Fabricator AAS-IR protocol.

use serde::{Serialize, Deserialize};
use std::convert::TryInto;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpCode {
    Nop = 0x00,
    LoadPlugin = 0x01,
    ExecuteCapability = 0x02,
    SyncFederation = 0x03,
    Hibernate = 0x04,
    EmitSignal = 0x05,
    MemoryRead = 0x06,
    MemoryWrite = 0x07,
    MetabolicUpdate = 0x08,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub opcode: OpCode,
    pub target: String,      // 32-byte fixed-width equivalent in binary
    pub capability: String,  // 32-byte fixed-width equivalent in binary
    pub payload: Vec<u8>,
}

impl Instruction {
    pub fn new(opcode: OpCode, target: &str, capability: &str, payload: Vec<u8>) -> Self {
        Self {
            opcode,
            target: target.to_string(),
            capability: capability.to_string(),
            payload,
        }
    }

    /// Encodes the instruction into the AAS-IR binary format:
    /// [OpCode: 2 bytes] [Target: 32 bytes] [Capability: 32 bytes] [PayloadLen: 4 bytes] [Payload: N bytes]
    pub fn to_binary(&self) -> Vec<u8> {
        let mut bin = Vec::with_capacity(70 + self.payload.len());
        
        // OpCode (Little Endian)
        bin.extend_from_slice(&(self.opcode as u16).to_le_bytes());
        
        // Target (Fixed 32 bytes)
        let mut target_bytes = [0u8; 32];
        let t_src = self.target.as_bytes();
        let t_len = t_src.len().min(32);
        target_bytes[..t_len].copy_from_slice(&t_src[..t_len]);
        bin.extend_from_slice(&target_bytes);
        
        // Capability (Fixed 32 bytes)
        let mut cap_bytes = [0u8; 32];
        let c_src = self.capability.as_bytes();
        let c_len = c_src.len().min(32);
        cap_bytes[..c_len].copy_from_slice(&c_src[..c_len]);
        bin.extend_from_slice(&cap_bytes);
        
        // Payload Length
        bin.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        
        // Payload
        bin.extend_from_slice(&self.payload);
        
        bin
    }

    /// Decodes an instruction from the AAS-IR binary format.
    pub fn from_binary(data: &[u8]) -> Result<Self, String> {
        if data.len() < 70 {
            return Err("Data too short for AAS-IR header".to_string());
        }
        
        let opcode_raw = u16::from_le_bytes(data[0..2].try_into().unwrap());
        let opcode = match opcode_raw {
            0 => OpCode::Nop,
            1 => OpCode::LoadPlugin,
            2 => OpCode::ExecuteCapability,
            3 => OpCode::SyncFederation,
            4 => OpCode::Hibernate,
            5 => OpCode::EmitSignal,
            6 => OpCode::MemoryRead,
            7 => OpCode::MemoryWrite,
            8 => OpCode::MetabolicUpdate,
            _ => return Err(format!("Unknown OpCode: 0x{:04X}", opcode_raw)),
        };
        
        let target = String::from_utf8_lossy(&data[2..34]).trim_matches('\0').to_string();
        let capability = String::from_utf8_lossy(&data[34..66]).trim_matches('\0').to_string();
        
        let payload_len = u32::from_le_bytes(data[66..70].try_into().unwrap()) as usize;
        if data.len() < 70 + payload_len {
            return Err("Data too short for AAS-IR payload".to_string());
        }
        
        let payload = data[70..70 + payload_len].to_vec();
        
        Ok(Self {
            opcode,
            target,
            capability,
            payload,
        })
    }
}
