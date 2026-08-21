use crate::{ChimeraIrInstruction, CirOpcode};

#[repr(C, align(64))]
pub struct InversionOptimizer {
    pub scale_cutoff_threshold: f32,
    pub minimized_instruction_count: u64,
}

impl InversionOptimizer {
    pub fn new(threshold: f32) -> Self {
        InversionOptimizer {
            scale_cutoff_threshold: threshold,
            minimized_instruction_count: 0,
        }
    }

    pub fn optimize_instruction_block(
        &mut self,
        input_buffer: &[ChimeraIrInstruction; 64],
        output_buffer: &mut [ChimeraIrInstruction; 64],
    ) -> usize {
        let mut continuous_writes = 0;
        let mut last_valid_load_mask: u64 = 0;

        for i in 0..64 {
            let current_inst = unsafe { *input_buffer.get_unchecked(i) };

            if current_inst.opcode == CirOpcode::MemoryLoad {
                if current_inst.source_register_mask == last_valid_load_mask {
                    continue;
                }
                last_valid_load_mask = current_inst.source_register_mask;
            }

            if current_inst.systemic_entropy_weight > self.scale_cutoff_threshold {
                continue;
            }

            unsafe {
                *output_buffer.get_unchecked_mut(continuous_writes) = current_inst;
            }
            
            continuous_writes += 1;
            self.minimized_instruction_count += 1;
        }

        continuous_writes
    }
}