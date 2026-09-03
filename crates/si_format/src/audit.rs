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

/// Hardware Target Architecture for JIT Code Auditing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetArch {
    X86_64,
    AArch64,
}

/// Pluggable Machine Code Security Auditor Trait
pub trait InstructionSetAuditor: Send + Sync {
    fn target_arch(&self) -> TargetArch;
    fn audit(&self, bytecode: &[u8]) -> Result<AuditResult>;
}

/// Security auditor for x86_64 machine code
pub struct X86_64Auditor;

impl InstructionSetAuditor for X86_64Auditor {
    fn target_arch(&self) -> TargetArch {
        TargetArch::X86_64
    }

    fn audit(&self, bytecode: &[u8]) -> Result<AuditResult> {
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
}

/// Security auditor for AArch64 (ARM64) machine code
pub struct AArch64Auditor;

impl InstructionSetAuditor for AArch64Auditor {
    fn target_arch(&self) -> TargetArch {
        TargetArch::AArch64
    }

    fn audit(&self, bytecode: &[u8]) -> Result<AuditResult> {
        if bytecode.is_empty() {
            return Ok(AuditResult::Allowed);
        }

        // AArch64 instructions are 32-bit (4-byte) aligned
        for chunk in bytecode.chunks_exact(4) {
            let inst = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);

            // SVC (Supervisor Call): bits [31:24] == 0b1101_0100, bits [23:21] == 0b000, bits [1:0] == 0b01
            if (inst & 0xFFE0_001F) == 0xD400_0001 {
                return Ok(AuditResult::Denied("Unauthorized AArch64 SVC system call".to_string()));
            }

            // HVC (Hypervisor Call): bits [1:0] == 0b10
            if (inst & 0xFFE0_001F) == 0xD400_0002 {
                return Ok(AuditResult::Denied("Unauthorized AArch64 HVC hypervisor call".to_string()));
            }

            // SMC (Secure Monitor Call): bits [1:0] == 0b11
            if (inst & 0xFFE0_001F) == 0xD400_0003 {
                return Ok(AuditResult::Denied("Unauthorized AArch64 SMC secure monitor call".to_string()));
            }

            // BRK (Breakpoint): bits [31:21] == 0b1101_0100_001, bits [4:0] == 0b00000
            if (inst & 0xFFE0_001F) == 0xD420_0000 {
                return Ok(AuditResult::Denied("Unauthorized AArch64 BRK breakpoint instruction".to_string()));
            }
        }

        Ok(AuditResult::Allowed)
    }
}

/// Audits a compiled native machine code buffer for forbidden instructions using the host architecture.
///
/// # Returns
/// - `Ok(AuditResult::Allowed)` if the buffer is clean.
/// - `Ok(AuditResult::Denied(reason))` if unauthorized opcodes are discovered.
pub fn audit(bytecode: &[u8]) -> Result<AuditResult> {
    X86_64Auditor.audit(bytecode)
}

/// Audits bytecode targeting a specific hardware architecture
pub fn audit_arch(bytecode: &[u8], arch: TargetArch) -> Result<AuditResult> {
    match arch {
        TargetArch::X86_64 => X86_64Auditor.audit(bytecode),
        TargetArch::AArch64 => AArch64Auditor.audit(bytecode),
    }
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

    #[test]
    fn test_audit_aarch64_svc_denied() {
        // AArch64 `svc #0` is 0xD4000001 (little-endian: 0x01, 0x00, 0x00, 0xD4)
        let svc_inst = vec![0x01, 0x00, 0x00, 0xD4];
        let res = audit_arch(&svc_inst, TargetArch::AArch64).unwrap();
        assert!(matches!(res, AuditResult::Denied(_)));
    }

    #[test]
    fn test_audit_aarch64_safe() {
        // AArch64 `nop` is 0xD503201F (little-endian: 0x1F, 0x20, 0x03, 0xD5)
        let nop_inst = vec![0x1F, 0x20, 0x03, 0xD5];
        let res = audit_arch(&nop_inst, TargetArch::AArch64).unwrap();
        assert_eq!(res, AuditResult::Allowed);
    }
}
