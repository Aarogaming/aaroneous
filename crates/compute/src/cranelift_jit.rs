//! crates/compute/src/cranelift_jit.rs
//! Cranelift Native Machine Code JIT Compiler Engine for Machine-Native IR (`si_ir`).
//! Translates discrete `NativeComputationalGraph` nodes and `MachineOpcode` variants
//! into optimized native host machine code (x86_64 / AArch64) in W^X executable memory regions.

use anyhow::{anyhow, Result};
use cranelift_codegen::ir::{
    types, AbiParam, Function, InstBuilder, MemFlags, Signature,
};
use cranelift_codegen::isa::{self, TargetIsa};
use cranelift_codegen::settings::{self, Configurable, Flags};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use si_ir::{MachineOpcode, NativeComputationalGraph};
use std::sync::Arc;

use crate::si_jit::NativeExecutionContext;
use crate::wx_memory::WxMemoryRegion;

/// Type definition for JIT-compiled native execution function
pub type NativeExecutionFn = unsafe extern "C" fn(*mut NativeExecutionContext) -> u64;

/// Cranelift Native JIT Compiler Engine
pub struct CraneliftJitEngine {
    isa: Arc<dyn TargetIsa>,
}

impl Default for CraneliftJitEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize default Cranelift host ISA")
    }
}

impl CraneliftJitEngine {
    /// Initializes Cranelift JIT engine for the current host architecture
    pub fn new() -> Result<Self> {
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed_and_size")
            .map_err(|e| anyhow!("Failed to set Cranelift opt_level: {e}"))?;
        flag_builder.set("is_pic", "true")
            .map_err(|e| anyhow!("Failed to set Cranelift is_pic: {e}"))?;

        let flags = Flags::new(flag_builder);
        let host_triple = target_lexicon::Triple::host();
        let isa = isa::lookup(host_triple)
            .map_err(|e| anyhow!("Failed to lookup host target ISA: {e}"))?
            .finish(flags)
            .map_err(|e| anyhow!("Failed to finish ISA configuration: {e}"))?;

        Ok(Self { isa })
    }

    /// Compiles a `NativeComputationalGraph` into a native executable `WxMemoryRegion`
    pub fn compile_graph_to_memory(
        &self,
        graph: &NativeComputationalGraph,
    ) -> Result<(WxMemoryRegion, NativeExecutionFn)> {
        let machine_code = self.compile_graph_to_bytes(graph)?;
        let memory_region = WxMemoryRegion::from_machine_code(&machine_code)?;
        let fn_ptr: NativeExecutionFn = unsafe { memory_region.as_fn_ptr() };
        Ok((memory_region, fn_ptr))
    }

    /// Compiles a `NativeComputationalGraph` into raw native machine code bytes
    pub fn compile_graph_to_bytes(&self, graph: &NativeComputationalGraph) -> Result<Vec<u8>> {
        let mut sig = Signature::new(self.isa.default_call_conv());
        // Argument 0: pointer to NativeExecutionContext (*mut NativeExecutionContext)
        sig.params.push(AbiParam::new(self.isa.pointer_type()));
        // Return value: u64
        sig.returns.push(AbiParam::new(types::I64));

        let mut func = Function::with_name_signature(
            cranelift_codegen::ir::UserFuncName::user(0, 0),
            sig,
        );

        let mut fn_builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let ctx_ptr = builder.block_params(entry_block)[0];

        let default_return_val = builder.ins().iconst(types::I64, 0);
        let mut returned = false;

        // Emit instructions for all nodes in the DAG
        for node in graph.nodes.values() {
            match &node.opcode {
                MachineOpcode::Alloc { size_bytes, .. } => {
                    // memory_pool is offset 128 in NativeExecutionContext
                    let pool_offset = builder.ins().iadd_imm(ctx_ptr, 128);
                    // Store pool_offset into registers[0] (offset 0)
                    builder.ins().store(MemFlags::trusted(), pool_offset, ctx_ptr, 0);

                    // Store size_bytes into registers[1] (offset 8)
                    let size_val = builder.ins().iconst(types::I64, *size_bytes as i64);
                    builder.ins().store(MemFlags::trusted(), size_val, ctx_ptr, 8);
                }
                MachineOpcode::Load { address_reg } => {
                    let addr_offset = ((*address_reg as i32) * 8).min(120);
                    let loaded_val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, addr_offset);
                    builder.ins().store(MemFlags::trusted(), loaded_val, ctx_ptr, 0);
                }
                MachineOpcode::Store { address_reg, value_reg } => {
                    let val_offset = ((*value_reg as i32) * 8).min(120);
                    let addr_offset = ((*address_reg as i32) * 8).min(120);
                    let val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, val_offset);
                    builder.ins().store(MemFlags::trusted(), val, ctx_ptr, addr_offset);
                }
                MachineOpcode::BranchIf { condition_reg, .. } => {
                    let cond_offset = ((*condition_reg as i32) * 8).min(120);
                    let cond_val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, cond_offset);
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_non_zero = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, cond_val, zero);

                    let then_block = builder.create_block();
                    let merge_block = builder.create_block();

                    builder.ins().brif(is_non_zero, then_block, &[], merge_block, &[]);
                    builder.switch_to_block(then_block);
                    builder.seal_block(then_block);

                    // Set status_code (offset 4224) to 1
                    let status_one = builder.ins().iconst(types::I32, 1);
                    builder.ins().store(MemFlags::trusted(), status_one, ctx_ptr, 4224);
                    builder.ins().jump(merge_block, &[]);

                    builder.switch_to_block(merge_block);
                    builder.seal_block(merge_block);
                }
                MachineOpcode::Call { function_id, arg_regs } => {
                    // Update status_code with low 32 bits of function_id
                    let fn_code = builder.ins().iconst(types::I32, (*function_id & 0xFFFFFFFF) as i64);
                    builder.ins().store(MemFlags::trusted(), fn_code, ctx_ptr, 4224);
                    if let Some(&first_arg) = arg_regs.first() {
                        let arg_offset = ((first_arg as i32) * 8).min(120);
                        let arg_val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, arg_offset);
                        builder.ins().store(MemFlags::trusted(), arg_val, ctx_ptr, 0);
                    }
                }
                MachineOpcode::TensorDot { left_reg, right_reg, .. } => {
                    let left_offset = ((*left_reg as i32) * 8).min(120);
                    let right_offset = ((*right_reg as i32) * 8).min(120);
                    let left_val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, left_offset);
                    let right_val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, right_offset);
                    let prod = builder.ins().imul(left_val, right_val);
                    builder.ins().store(MemFlags::trusted(), prod, ctx_ptr, 0);
                }
                MachineOpcode::EntropyMinimization { state_reg } => {
                    let reg_offset = ((*state_reg as i32) * 8).min(120);
                    let val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, reg_offset);
                    let minimized = builder.ins().ushr_imm(val, 1);
                    builder.ins().store(MemFlags::trusted(), minimized, ctx_ptr, reg_offset);
                }
                MachineOpcode::Return { value_reg } => {
                    let ret_offset = ((*value_reg as i32) * 8).min(120);
                    let ret_val = builder.ins().load(types::I64, MemFlags::trusted(), ctx_ptr, ret_offset);
                    builder.ins().return_(&[ret_val]);
                    returned = true;
                    break;
                }
            }
        }

        if !returned {
            builder.ins().return_(&[default_return_val]);
        }

        builder.finalize();

        // Compile to machine code bytes
        let mut ctx = Context::for_function(func);
        let compiled = ctx
            .compile(self.isa.as_ref(), &mut Default::default())
            .map_err(|e| anyhow!("Cranelift compilation error: {:?}", e))?;

        let code_buffer = compiled.code_buffer();
        Ok(code_buffer.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_ir::{NativeComputationNode, NativeTypeLattice};

    #[test]
    fn test_cranelift_jit_compilation_and_execution() {
        let jit = CraneliftJitEngine::new().unwrap();
        let mut graph = NativeComputationalGraph::new();

        // Node 1: Alloc 2048 bytes (writes pool ptr to reg[0], 2048 to reg[1])
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 2048, align: 64 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 64 },
            energy_cost: 0.05,
            dependencies: vec![],
        });

        // Node 2: Return value from reg[1] (expected: 2048)
        graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Return { value_reg: 1 },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 64, signed: false },
            energy_cost: 0.01,
            dependencies: vec![1],
        });

        let (_region, func) = jit.compile_graph_to_memory(&graph).unwrap();

        let mut ctx = NativeExecutionContext::default();
        let result = unsafe { func(&mut ctx) };

        assert_eq!(result, 2048);
        assert_eq!(ctx.registers[1], 2048);
    }

    #[test]
    fn test_cranelift_jit_tensordot_multiplication() {
        let jit = CraneliftJitEngine::new().unwrap();
        let mut graph = NativeComputationalGraph::new();

        // Node 1: Store reg[1] * reg[2] -> reg[0]
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::TensorDot { left_reg: 1, right_reg: 2, dim: 64 },
            type_lattice: NativeTypeLattice::TensorType {
                shape: vec![64],
                element_type: Box::new(NativeTypeLattice::PrimitiveFloat { bits: 32 }),
            },
            energy_cost: 0.10,
            dependencies: vec![],
        });

        // Node 2: Return reg[0]
        graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Return { value_reg: 0 },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 64, signed: false },
            energy_cost: 0.01,
            dependencies: vec![1],
        });

        let (_region, func) = jit.compile_graph_to_memory(&graph).unwrap();

        let mut ctx = NativeExecutionContext::default();
        ctx.registers[1] = 7;
        ctx.registers[2] = 6;

        let result = unsafe { func(&mut ctx) };
        assert_eq!(result, 42);
        assert_eq!(ctx.registers[0], 42);
    }
}
