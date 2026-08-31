//! # Sandboxed Micro-Worker Bytecode Virtual Machine
//!
//! Pure-Rust, zero-`unsafe`, deterministic register-based VM for executing untrusted
//! user plugins and dynamic hypervisor micro-tasks with instruction gas metering and bounded memory.

use serde::{Deserialize, Serialize};

/// Number of general-purpose 64-bit registers (r0 through r15)
pub const REGISTER_COUNT: usize = 16;
/// Default memory capacity (64 KB)
pub const DEFAULT_MEMORY_LIMIT: usize = 64 * 1024;
/// Default gas allocation (100,000 instructions)
pub const DEFAULT_GAS_LIMIT: u64 = 100_000;

/// Errors that can occur during bytecode VM execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmError {
    GasExhausted,
    MemoryOutOfBounds { address: usize, limit: usize },
    InvalidRegister(u8),
    InvalidOpcode(u8),
    DivisionByZero,
    ProgramCounterOutOfBounds { pc: usize, len: usize },
    ProgramHalted,
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::GasExhausted => write!(f, "Execution gas exhausted"),
            VmError::MemoryOutOfBounds { address, limit } => {
                write!(f, "Memory access out of bounds: address {} >= limit {}", address, limit)
            }
            VmError::InvalidRegister(reg) => write!(f, "Invalid register index: r{}", reg),
            VmError::InvalidOpcode(op) => write!(f, "Invalid instruction opcode: 0x{:02X}", op),
            VmError::DivisionByZero => write!(f, "Division by zero in arithmetic operation"),
            VmError::ProgramCounterOutOfBounds { pc, len } => {
                write!(f, "Program counter out of bounds: pc {} >= len {}", pc, len)
            }
            VmError::ProgramHalted => write!(f, "Program execution halted"),
        }
    }
}

impl std::error::Error for VmError {}

/// VM Bytecode Instruction Set
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmInstruction {
    Nop,
    MovImm { dst: u8, val: i64 },
    MovReg { dst: u8, src: u8 },
    Add { dst: u8, src: u8 },
    Sub { dst: u8, src: u8 },
    Mul { dst: u8, src: u8 },
    Div { dst: u8, src: u8 },
    Mod { dst: u8, src: u8 },
    BitAnd { dst: u8, src: u8 },
    BitOr { dst: u8, src: u8 },
    BitXor { dst: u8, src: u8 },
    Shl { dst: u8, shift: u8 },
    Shr { dst: u8, shift: u8 },
    LoadMem { dst: u8, addr_reg: u8 },
    StoreMem { src: u8, addr_reg: u8 },
    Jmp { target: usize },
    JmpIfZero { cond_reg: u8, target: usize },
    JmpIfNotZero { cond_reg: u8, target: usize },
    JmpIfGreater { reg_a: u8, reg_b: u8, target: usize },
    Halt,
}

/// Compiled bytecode program
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmProgram {
    pub instructions: Vec<VmInstruction>,
    pub initial_memory: Vec<u8>,
}

impl VmProgram {
    pub fn new(instructions: Vec<VmInstruction>) -> Self {
        Self {
            instructions,
            initial_memory: Vec::new(),
        }
    }

    pub fn with_memory(instructions: Vec<VmInstruction>, initial_memory: Vec<u8>) -> Self {
        Self {
            instructions,
            initial_memory,
        }
    }
}

/// Result of VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmExecutionResult {
    pub registers: [i64; REGISTER_COUNT],
    pub gas_consumed: u64,
    pub gas_remaining: u64,
    pub instructions_executed: usize,
    pub memory_snapshot: Vec<u8>,
}

/// Sandboxed Micro-Worker Bytecode Virtual Machine
pub struct MicroBytecodeVm {
    registers: [i64; REGISTER_COUNT],
    pc: usize,
    memory: Vec<u8>,
    memory_limit: usize,
    gas_remaining: u64,
    gas_consumed: u64,
    instructions_executed: usize,
}

impl MicroBytecodeVm {
    /// Creates a new VM with default gas and memory limits
    pub fn new() -> Self {
        Self::with_limits(DEFAULT_GAS_LIMIT, DEFAULT_MEMORY_LIMIT)
    }

    /// Creates a new VM with custom gas and memory constraints
    pub fn with_limits(gas_limit: u64, memory_limit: usize) -> Self {
        Self {
            registers: [0; REGISTER_COUNT],
            pc: 0,
            memory: vec![0; memory_limit],
            memory_limit,
            gas_remaining: gas_limit,
            gas_consumed: 0,
            instructions_executed: 0,
        }
    }

    /// Resets the VM state for executing a new program
    pub fn reset(&mut self, gas_limit: u64) {
        self.registers = [0; REGISTER_COUNT];
        self.pc = 0;
        self.memory.fill(0);
        self.gas_remaining = gas_limit;
        self.gas_consumed = 0;
        self.instructions_executed = 0;
    }

    fn check_reg(&self, reg: u8) -> Result<usize, VmError> {
        let idx = reg as usize;
        if idx < REGISTER_COUNT {
            Ok(idx)
        } else {
            Err(VmError::InvalidRegister(reg))
        }
    }

    fn consume_gas(&mut self, amount: u64) -> Result<(), VmError> {
        if self.gas_remaining >= amount {
            self.gas_remaining -= amount;
            self.gas_consumed += amount;
            Ok(())
        } else {
            self.gas_consumed += self.gas_remaining;
            self.gas_remaining = 0;
            Err(VmError::GasExhausted)
        }
    }

    /// Executes a bytecode program within the sandboxed VM environment
    pub fn execute(&mut self, program: &VmProgram) -> Result<VmExecutionResult, VmError> {
        // Load initial data into memory if provided
        if !program.initial_memory.is_empty() {
            let copy_len = program.initial_memory.len().min(self.memory_limit);
            self.memory[..copy_len].copy_from_slice(&program.initial_memory[..copy_len]);
        }

        let instructions_len = program.instructions.len();

        while self.pc < instructions_len {
            self.consume_gas(1)?;
            self.instructions_executed += 1;

            let inst = match program.instructions.get(self.pc) {
                Some(i) => i,
                None => {
                    return Err(VmError::ProgramCounterOutOfBounds {
                        pc: self.pc,
                        len: instructions_len,
                    });
                }
            };

            match inst {
                VmInstruction::Nop => {
                    self.pc += 1;
                }
                VmInstruction::MovImm { dst, val } => {
                    let d = self.check_reg(*dst)?;
                    self.registers[d] = *val;
                    self.pc += 1;
                }
                VmInstruction::MovReg { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    self.registers[d] = self.registers[s];
                    self.pc += 1;
                }
                VmInstruction::Add { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    self.registers[d] = self.registers[d].wrapping_add(self.registers[s]);
                    self.pc += 1;
                }
                VmInstruction::Sub { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    self.registers[d] = self.registers[d].wrapping_sub(self.registers[s]);
                    self.pc += 1;
                }
                VmInstruction::Mul { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    self.registers[d] = self.registers[d].wrapping_mul(self.registers[s]);
                    self.pc += 1;
                }
                VmInstruction::Div { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    let divisor = self.registers[s];
                    if divisor == 0 {
                        return Err(VmError::DivisionByZero);
                    }
                    self.registers[d] = self.registers[d].wrapping_div(divisor);
                    self.pc += 1;
                }
                VmInstruction::Mod { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    let divisor = self.registers[s];
                    if divisor == 0 {
                        return Err(VmError::DivisionByZero);
                    }
                    self.registers[d] = self.registers[d].wrapping_rem(divisor);
                    self.pc += 1;
                }
                VmInstruction::BitAnd { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    self.registers[d] &= self.registers[s];
                    self.pc += 1;
                }
                VmInstruction::BitOr { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    self.registers[d] |= self.registers[s];
                    self.pc += 1;
                }
                VmInstruction::BitXor { dst, src } => {
                    let d = self.check_reg(*dst)?;
                    let s = self.check_reg(*src)?;
                    self.registers[d] ^= self.registers[s];
                    self.pc += 1;
                }
                VmInstruction::Shl { dst, shift } => {
                    let d = self.check_reg(*dst)?;
                    self.registers[d] = self.registers[d].wrapping_shl(*shift as u32);
                    self.pc += 1;
                }
                VmInstruction::Shr { dst, shift } => {
                    let d = self.check_reg(*dst)?;
                    self.registers[d] = self.registers[d].wrapping_shr(*shift as u32);
                    self.pc += 1;
                }
                VmInstruction::LoadMem { dst, addr_reg } => {
                    let d = self.check_reg(*dst)?;
                    let a = self.check_reg(*addr_reg)?;
                    let addr = self.registers[a] as usize;
                    if addr + 8 <= self.memory_limit {
                        let bytes: [u8; 8] = match self.memory[addr..addr + 8].try_into() {
                            Ok(b) => b,
                            Err(_) => {
                                return Err(VmError::MemoryOutOfBounds {
                                    address: addr,
                                    limit: self.memory_limit,
                                });
                            }
                        };
                        self.registers[d] = i64::from_le_bytes(bytes);
                        self.pc += 1;
                    } else {
                        return Err(VmError::MemoryOutOfBounds {
                            address: addr,
                            limit: self.memory_limit,
                        });
                    }
                }
                VmInstruction::StoreMem { src, addr_reg } => {
                    let s = self.check_reg(*src)?;
                    let a = self.check_reg(*addr_reg)?;
                    let addr = self.registers[a] as usize;
                    if addr + 8 <= self.memory_limit {
                        let bytes = self.registers[s].to_le_bytes();
                        self.memory[addr..addr + 8].copy_from_slice(&bytes);
                        self.pc += 1;
                    } else {
                        return Err(VmError::MemoryOutOfBounds {
                            address: addr,
                            limit: self.memory_limit,
                        });
                    }
                }
                VmInstruction::Jmp { target } => {
                    if *target < instructions_len {
                        self.pc = *target;
                    } else {
                        return Err(VmError::ProgramCounterOutOfBounds {
                            pc: *target,
                            len: instructions_len,
                        });
                    }
                }
                VmInstruction::JmpIfZero { cond_reg, target } => {
                    let c = self.check_reg(*cond_reg)?;
                    if self.registers[c] == 0 {
                        if *target < instructions_len {
                            self.pc = *target;
                        } else {
                            return Err(VmError::ProgramCounterOutOfBounds {
                                pc: *target,
                                len: instructions_len,
                            });
                        }
                    } else {
                        self.pc += 1;
                    }
                }
                VmInstruction::JmpIfNotZero { cond_reg, target } => {
                    let c = self.check_reg(*cond_reg)?;
                    if self.registers[c] != 0 {
                        if *target < instructions_len {
                            self.pc = *target;
                        } else {
                            return Err(VmError::ProgramCounterOutOfBounds {
                                pc: *target,
                                len: instructions_len,
                            });
                        }
                    } else {
                        self.pc += 1;
                    }
                }
                VmInstruction::JmpIfGreater { reg_a, reg_b, target } => {
                    let a = self.check_reg(*reg_a)?;
                    let b = self.check_reg(*reg_b)?;
                    if self.registers[a] > self.registers[b] {
                        if *target < instructions_len {
                            self.pc = *target;
                        } else {
                            return Err(VmError::ProgramCounterOutOfBounds {
                                pc: *target,
                                len: instructions_len,
                            });
                        }
                    } else {
                        self.pc += 1;
                    }
                }
                VmInstruction::Halt => {
                    break;
                }
            }
        }

        Ok(VmExecutionResult {
            registers: self.registers,
            gas_consumed: self.gas_consumed,
            gas_remaining: self.gas_remaining,
            instructions_executed: self.instructions_executed,
            memory_snapshot: self.memory.clone(),
        })
    }
}

impl Default for MicroBytecodeVm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic_execution() {
        let mut vm = MicroBytecodeVm::new();
        let program = VmProgram::new(vec![
            VmInstruction::MovImm { dst: 0, val: 42 },
            VmInstruction::MovImm { dst: 1, val: 58 },
            VmInstruction::Add { dst: 0, src: 1 },
            VmInstruction::Halt,
        ]);

        let result = vm.execute(&program).unwrap();
        assert_eq!(result.registers[0], 100);
        assert_eq!(result.instructions_executed, 4);
    }

    #[test]
    fn test_loop_and_jump_countdown() {
        let mut vm = MicroBytecodeVm::new();
        // Compute sum(10..1)
        let program = VmProgram::new(vec![
            VmInstruction::MovImm { dst: 0, val: 0 },   // r0 = accumulator (0)
            VmInstruction::MovImm { dst: 1, val: 10 },  // r1 = counter (10)
            VmInstruction::MovImm { dst: 2, val: 1 },   // r2 = step (1)
            // Loop start (pc = 3)
            VmInstruction::Add { dst: 0, src: 1 },      // r0 += r1
            VmInstruction::Sub { dst: 1, src: 2 },      // r1 -= 1
            VmInstruction::JmpIfNotZero { cond_reg: 1, target: 3 }, // if r1 != 0 goto 3
            VmInstruction::Halt,
        ]);

        let result = vm.execute(&program).unwrap();
        assert_eq!(result.registers[0], 55); // 10+9+8+7+6+5+4+3+2+1 = 55
        assert_eq!(result.registers[1], 0);
    }

    #[test]
    fn test_gas_exhaustion_protection() {
        let mut vm = MicroBytecodeVm::with_limits(50, DEFAULT_MEMORY_LIMIT);
        // Infinite loop: jmp 0
        let program = VmProgram::new(vec![
            VmInstruction::MovImm { dst: 0, val: 1 },
            VmInstruction::Jmp { target: 0 },
        ]);

        let result = vm.execute(&program);
        assert!(matches!(result, Err(VmError::GasExhausted)));
        assert_eq!(vm.gas_consumed, 50);
        assert_eq!(vm.gas_remaining, 0);
    }

    #[test]
    fn test_memory_out_of_bounds_protection() {
        let mut vm = MicroBytecodeVm::with_limits(1000, 1024);
        let program = VmProgram::new(vec![
            VmInstruction::MovImm { dst: 0, val: 99999 }, // Far out of 1024B bounds
            VmInstruction::LoadMem { dst: 1, addr_reg: 0 },
        ]);

        let result = vm.execute(&program);
        assert!(matches!(
            result,
            Err(VmError::MemoryOutOfBounds {
                address: 99999,
                limit: 1024,
            })
        ));
    }

    #[test]
    fn test_division_by_zero_safety() {
        let mut vm = MicroBytecodeVm::new();
        let program = VmProgram::new(vec![
            VmInstruction::MovImm { dst: 0, val: 100 },
            VmInstruction::MovImm { dst: 1, val: 0 },
            VmInstruction::Div { dst: 0, src: 1 },
        ]);

        let result = vm.execute(&program);
        assert!(matches!(result, Err(VmError::DivisionByZero)));
    }
}
