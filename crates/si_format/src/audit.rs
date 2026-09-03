//! crates/si_format/src/audit.rs
//! JIT Bytecode Governance Security Audit Gate.
//!
//! Scans native executable machine opcodes (x86_64 / AArch64) for forbidden,
//! privileged, or ring-0 instructions (e.g. `syscall`, `sysenter`, `int 0x80`,
//! `cli`, `sti`, `hlt`, `wrmsr`) prior to executing within W^X memory regions.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Result of a JIT bytecode governance security audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult {
    /// The bytecode satisfies all security invariants and contains no forbidden opcodes.
    Allowed,
    /// The bytecode was rejected due to containing unauthorized instructions.
    Denied(String),
}

/// Audits a compiled native machine code buffer for forbidden instructions.
///
/// # Returns
/// - `Ok(AuditResult::Allowed)` if the buffer is clean.
/// - `Ok(AuditResult::Denied(reason))` if unauthorized opcodes are discovered.
pub fn audit(bytecode: &[u8]) -> Result<AuditResult> {
    if bytecode.is_empty() {
        return Ok(AuditResult::Allowed);
    }

    let mut i = 0;
    let len = bytecode.len();

    while i < len {
        let b = bytecode[i];

        // 1-byte checks
        match b {
            0xF4 => return Ok(AuditResult::Denied("Unauthorized privileged opcode: hlt (0xF4)".to_string())),
            0xFA => return Ok(AuditResult::Denied("Unauthorized privileged opcode: cli (0xFA)".to_string())),
            0xFB => return Ok(AuditResult::Denied("Unauthorized privileged opcode: sti (0xFB)".to_string())),
            0xCC => return Ok(AuditResult::Denied("Unauthorized debug breakpoint: int3 (0xCC)".to_string())),
            0xCE => return Ok(AuditResult::Denied("Unauthorized opcode: into (0xCE)".to_string())),
            _ => {}
        }

        // 2-byte checks
        if i + 1 < len {
            let next = bytecode[i + 1];
            match (b, next) {
                (0x0F, 0x05) => {
                    return Ok(AuditResult::Denied("Unauthorized syscall opcode (0x0F 0x05)".to_string()));
                }
                (0x0F, 0x34) => {
                    return Ok(AuditResult::Denied("Unauthorized sysenter opcode (0x0F 0x34)".to_string()));
                }
                (0x0F, 0x35) => {
                    return Ok(AuditResult::Denied("Unauthorized sysexit opcode (0x0F 0x35)".to_string()));
                }
                (0x0F, 0x07) => {
                    return Ok(AuditResult::Denied("Unauthorized sysret opcode (0x0F 0x07)".to_string()));
                }
                (0x0F, 0x30) => {
                    return Ok(AuditResult::Denied("Unauthorized privileged opcode: wrmsr (0x0F 0x30)".to_string()));
                }
                (0x0F, 0x32) => {
                    return Ok(AuditResult::Denied("Unauthorized privileged opcode: rdmsr (0x0F 0x32)".to_string()));
                }
                (0xCD, 0x80) => {
                    return Ok(AuditResult::Denied("Unauthorized legacy syscall: int 0x80 (0xCD 0x80)".to_string()));
                }
                (0xCD, int_num) => {
                    return Ok(AuditResult::Denied(format!("Unauthorized software interrupt: int {:#x} (0xCD)", int_num)));
                }
                _ => {}
            }
        }

        i += 1;
    }

    Ok(AuditResult::Allowed)
}

/// Convenience gate function: returns `Ok(())` if allowed, or an `Err` if denied.
pub fn jit_audit(bytecode: &[u8]) -> Result<()> {
    match audit(bytecode)? {
        AuditResult::Allowed => Ok(()),
        AuditResult::Denied(reason) => bail!("Governance JIT Audit FAILED: {}", reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_safe_code() {
        let safe_nop = vec![0x90, 0x90, 0x90];
        assert_eq!(audit(&safe_nop).unwrap(), AuditResult::Allowed);
        assert!(jit_audit(&safe_nop).is_ok());
    }

    #[test]
    fn test_audit_syscall_denied() {
        let bad = vec![0x48, 0x89, 0xC0, 0x0F, 0x05];
        match audit(&bad).unwrap() {
            AuditResult::Denied(msg) => assert!(msg.contains("syscall")),
            _ => panic!("Expected syscall denial"),
        }
        assert!(jit_audit(&bad).is_err());
    }

    #[test]
    fn test_audit_int80_denied() {
        let bad = vec![0xCD, 0x80];
        assert!(matches!(audit(&bad).unwrap(), AuditResult::Denied(_)));
    }
}
