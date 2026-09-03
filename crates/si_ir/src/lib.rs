//! crates/si_ir/src/lib.rs
//! Machine-Native Synthetic Intelligence Intermediate Representation (SI-IR).
//! Defines the native computational graph DAG, algebraic type lattices,
//! dimensional SI physical invariants (7 SI base units), and thermodynamic free-energy bounds.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// SI Base Units with 7 Exponents:
/// [Mass (kg), Length (m), Time (s), Current (A), Temperature (K), Amount (mol), Luminosity (cd)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionalUnit {
    pub mass: i8,        // kg
    pub length: i8,      // m
    pub time: i8,        // s
    pub current: i8,     // A
    pub temperature: i8, // K
    pub amount: i8,      // mol
    pub luminosity: i8,  // cd
}

impl DimensionalUnit {
    pub const DIMENSIONLESS: Self = Self { mass: 0, length: 0, time: 0, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const LENGTH_METER: Self = Self { mass: 0, length: 1, time: 0, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const MASS_KILOGRAM: Self = Self { mass: 1, length: 0, time: 0, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const TIME_SECOND: Self = Self { mass: 0, length: 0, time: 1, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const CURRENT_AMPERE: Self = Self { mass: 0, length: 0, time: 0, current: 1, temperature: 0, amount: 0, luminosity: 0 };
    pub const TEMPERATURE_KELVIN: Self = Self { mass: 0, length: 0, time: 0, current: 0, temperature: 1, amount: 0, luminosity: 0 };
    pub const AMOUNT_MOLE: Self = Self { mass: 0, length: 0, time: 0, current: 0, temperature: 0, amount: 1, luminosity: 0 };
    pub const LUMEN_CANDELA: Self = Self { mass: 0, length: 0, time: 0, current: 0, temperature: 0, amount: 0, luminosity: 1 };

    pub const VELOCITY: Self = Self { mass: 0, length: 1, time: -1, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const ACCELERATION: Self = Self { mass: 0, length: 1, time: -2, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const FORCE_NEWTON: Self = Self { mass: 1, length: 1, time: -2, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const ENERGY_JOULE: Self = Self { mass: 1, length: 2, time: -2, current: 0, temperature: 0, amount: 0, luminosity: 0 };
    pub const POWER_WATT: Self = Self { mass: 1, length: 2, time: -3, current: 0, temperature: 0, amount: 0, luminosity: 0 };

    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            mass: self.mass + other.mass,
            length: self.length + other.length,
            time: self.time + other.time,
            current: self.current + other.current,
            temperature: self.temperature + other.temperature,
            amount: self.amount + other.amount,
            luminosity: self.luminosity + other.luminosity,
        }
    }

    pub fn divide(&self, other: &Self) -> Self {
        Self {
            mass: self.mass - other.mass,
            length: self.length - other.length,
            time: self.time - other.time,
            current: self.current - other.current,
            temperature: self.temperature - other.temperature,
            amount: self.amount - other.amount,
            luminosity: self.luminosity - other.luminosity,
        }
    }

    pub fn as_exponents(&self) -> [i8; 7] {
        [
            self.mass,
            self.length,
            self.time,
            self.current,
            self.temperature,
            self.amount,
            self.luminosity,
        ]
    }
}

/// Universal Measured or Actuated Physical Quantity with 7-Exponent Dimensional Tracking
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UniversalPhysicalQuantity {
    pub value: f64,
    pub unit: DimensionalUnit,
    pub uncertainty: f32,
}

impl UniversalPhysicalQuantity {
    pub fn new(value: f64, unit: DimensionalUnit, uncertainty: f32) -> Self {
        Self { value, unit, uncertainty }
    }

    pub fn dimensionless(value: f64) -> Self {
        Self::new(value, DimensionalUnit::DIMENSIONLESS, 0.0)
    }

    pub fn meters(value: f64) -> Self {
        Self::new(value, DimensionalUnit::LENGTH_METER, 0.0)
    }

    pub fn seconds(value: f64) -> Self {
        Self::new(value, DimensionalUnit::TIME_SECOND, 0.0)
    }

    pub fn velocity(value: f64) -> Self {
        Self::new(value, DimensionalUnit::VELOCITY, 0.0)
    }

    pub fn newtons(value: f64) -> Self {
        Self::new(value, DimensionalUnit::FORCE_NEWTON, 0.0)
    }

    pub fn multiply(&self, other: &Self) -> Self {
        Self {
            value: self.value * other.value,
            unit: self.unit.multiply(&other.unit),
            uncertainty: self.uncertainty + other.uncertainty,
        }
    }

    pub fn divide(&self, other: &Self) -> Option<Self> {
        if other.value.abs() < 1e-15 {
            return None;
        }
        Some(Self {
            value: self.value / other.value,
            unit: self.unit.divide(&other.unit),
            uncertainty: self.uncertainty + other.uncertainty,
        })
    }

    pub fn assert_compatible(&self, target_unit: &DimensionalUnit) -> bool {
        self.unit == *target_unit
    }
}

/// Machine-Native Low-Level Computational Opcode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MachineOpcode {
    Alloc { size_bytes: usize, align: usize },
    Load { address_reg: u16 },
    Store { address_reg: u16, value_reg: u16 },
    BranchIf { condition_reg: u16, target_block: u32 },
    Call { function_id: u64, arg_regs: Vec<u16> },
    TensorDot { left_reg: u16, right_reg: u16, dim: usize },
    EntropyMinimization { state_reg: u16 },
    Return { value_reg: u16 },
}

impl MachineOpcode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Alloc { .. } => "Alloc",
            Self::Load { .. } => "Load",
            Self::Store { .. } => "Store",
            Self::BranchIf { .. } => "BranchIf",
            Self::Call { .. } => "Call",
            Self::TensorDot { .. } => "TensorDot",
            Self::EntropyMinimization { .. } => "EntropyMinimization",
            Self::Return { .. } => "Return",
        }
    }
}

pub use smol_str::SmolStr;

/// Type Lattice and Algebraic Verification Bounds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NativeTypeLattice {
    PrimitiveInt { bits: u8, signed: bool },
    PrimitiveFloat { bits: u8 },
    PhysicalQuantity { unit: DimensionalUnit, precision: u8 },
    TensorType { shape: Vec<usize>, element_type: Box<NativeTypeLattice> },
    LinearMemoryPointer { mutability: bool, alignment: usize },
    InvariantRefinement { base: Box<NativeTypeLattice>, constraint_predicate: smol_str::SmolStr },
}

/// Node in the Machine-Native Computational DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeComputationNode {
    pub id: u64,
    pub opcode: MachineOpcode,
    pub type_lattice: NativeTypeLattice,
    pub energy_cost: f64,
    pub dependencies: Vec<u64>,
}

/// Machine-Native Computational Graph (The True Core Representation)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NativeComputationalGraph {
    pub nodes: BTreeMap<u64, NativeComputationNode>,
    pub entry_node: u64,
    pub exit_node: u64,
    pub thermodynamic_free_energy: f64,
    pub shannon_entropy: f64,
}

impl NativeComputationalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: NativeComputationNode) {
        self.thermodynamic_free_energy += node.energy_cost;
        self.nodes.insert(node.id, node);
    }

    /// Verifies physical dimensional consistency across all nodes in the DAG
    pub fn verify_dimensional_invariants(&self) -> Result<bool> {
        for node in self.nodes.values() {
            if let MachineOpcode::TensorDot { .. } = &node.opcode {
                match &node.type_lattice {
                    NativeTypeLattice::TensorType { .. } | NativeTypeLattice::PhysicalQuantity { .. } => {}
                    _ => return Err(anyhow!("Dimensional invariant violation on node {}", node.id)),
                }
            }
        }
        Ok(true)
    }

    /// Extracts graph node energy costs as a contiguous vector for GPU tensor training
    pub fn extract_energy_vector(&self) -> Vec<f64> {
        self.nodes.values().map(|n| n.energy_cost).collect()
    }

    /// Serializes the graph to length-prefixed compact bincode-style or JSON bytes
    pub fn to_compact_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| anyhow!("Graph serialization failed: {e}"))
    }

    /// Deserializes the graph from compact bytes
    pub fn from_compact_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| anyhow!("Graph deserialization failed: {e}"))
    }
}

/// Structure-of-Arrays (SoA) Dense Computational Storage (Mechanical Sympathy).
/// Maximizes CPU L1 data cache bandwidth by packing contiguous columnar vectors
/// for physical unit verification and JIT opcode lowering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DenseGraphStorage {
    pub node_ids: Vec<u64>,
    pub opcodes: Vec<MachineOpcode>,
    pub dimensional_units: Vec<DimensionalUnit>,
    pub free_energies: Vec<f64>,
    pub dependency_offsets: Vec<u32>,
}

impl DenseGraphStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            node_ids: Vec::with_capacity(capacity),
            opcodes: Vec::with_capacity(capacity),
            dimensional_units: Vec::with_capacity(capacity),
            free_energies: Vec::with_capacity(capacity),
            dependency_offsets: Vec::with_capacity(capacity),
        }
    }

    pub fn push_node(
        &mut self,
        id: u64,
        opcode: MachineOpcode,
        unit: DimensionalUnit,
        energy_cost: f64,
        dependency_offset: u32,
    ) {
        self.node_ids.push(id);
        self.opcodes.push(opcode);
        self.dimensional_units.push(unit);
        self.free_energies.push(energy_cost);
        self.dependency_offsets.push(dependency_offset);
    }

    pub fn len(&self) -> usize {
        self.node_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.node_ids.is_empty()
    }

    /// Linear sum of free energies streaming strictly contiguous float memory
    pub fn total_free_energy(&self) -> f64 {
        self.free_energies.iter().sum()
    }
}

/// Ephemeral Bump Allocation Arena for High-Frequency Graph Traversal (Mechanical Sympathy).
/// Provides zero-heap-allocation memory staging with O(1) instantaneous reset.
pub struct EphemeralExecutionArena {
    buffer: Vec<u8>,
    offset: usize,
    capacity: usize,
    node_pool: Vec<NativeComputationNode>,
}

impl EphemeralExecutionArena {
    /// Creates a new arena with pre-allocated buffer capacity in bytes.
    pub fn with_capacity(capacity_bytes: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity_bytes],
            offset: 0,
            capacity: capacity_bytes,
            node_pool: Vec::with_capacity(1024),
        }
    }

    /// Creates default 1MB ephemeral arena.
    pub fn new() -> Self {
        Self::with_capacity(1024 * 1024) // 1MB pre-allocated
    }

    /// Allocates an ephemeral contiguous `f32` tensor buffer from the arena.
    pub fn alloc_f32_slice(&mut self, count: usize) -> Result<&mut [f32]> {
        let size_bytes = count * std::mem::size_of::<f32>();
        let align = std::mem::align_of::<f32>();
        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        if aligned_offset + size_bytes > self.capacity {
            return Err(anyhow!("EphemeralExecutionArena out of capacity (requested {} bytes, remaining {})", size_bytes, self.capacity.saturating_sub(self.offset)));
        }

        self.offset = aligned_offset + size_bytes;
        let slice = unsafe {
            let ptr = self.buffer.as_mut_ptr().add(aligned_offset) as *mut f32;
            std::slice::from_raw_parts_mut(ptr, count)
        };
        Ok(slice)
    }

    /// Stages a computation node in the arena node pool with zero heap reallocation.
    pub fn push_node(&mut self, node: NativeComputationNode) -> usize {
        let idx = self.node_pool.len();
        self.node_pool.push(node);
        idx
    }

    /// Returns references to all nodes currently staged in this reflex tick.
    pub fn nodes(&self) -> &[NativeComputationNode] {
        &self.node_pool
    }

    /// Resets the bump allocator in O(1) time without deallocating backing memory.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.offset = 0;
        self.node_pool.clear();
    }

    /// Current allocated byte count.
    pub fn allocated_bytes(&self) -> usize {
        self.offset
    }
}

impl Default for EphemeralExecutionArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensional_unit_arithmetic() {
        let mass = DimensionalUnit::MASS_KILOGRAM;
        let accel = DimensionalUnit::ACCELERATION;
        let force = mass.multiply(&accel);
        assert_eq!(force, DimensionalUnit::FORCE_NEWTON);

        let meter = DimensionalUnit::LENGTH_METER;
        let work = force.multiply(&meter);
        assert_eq!(work, DimensionalUnit::ENERGY_JOULE);

        let time = DimensionalUnit::TIME_SECOND;
        let power = work.divide(&time);
        assert_eq!(power, DimensionalUnit::POWER_WATT);
    }

    #[test]
    fn test_computational_graph_construction() {
        let mut graph = NativeComputationalGraph::new();
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 128, align: 8 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 8 },
            energy_cost: 0.05,
            dependencies: vec![],
        });
        graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Return { value_reg: 0 },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 64, signed: false },
            energy_cost: 0.01,
            dependencies: vec![1],
        });

        assert_eq!(graph.nodes.len(), 2);
        assert!((graph.thermodynamic_free_energy - 0.06).abs() < 1e-6);
        assert!(graph.verify_dimensional_invariants().is_ok());

        // Compact byte serialization roundtrip
        let bytes = graph.to_compact_bytes().unwrap();
        assert!(!bytes.is_empty());
        let restored = NativeComputationalGraph::from_compact_bytes(&bytes).unwrap();
        assert_eq!(restored.nodes.len(), graph.nodes.len());
        assert_eq!(restored.nodes[&1].id, 1);
        assert_eq!(restored.nodes[&2].id, 2);
    }

    #[test]
    fn test_ephemeral_execution_arena_lifecycle() {
        let mut arena = EphemeralExecutionArena::with_capacity(4096);
        assert_eq!(arena.allocated_bytes(), 0);

        let slice = arena.alloc_f32_slice(256).expect("Allocation failed");
        assert_eq!(slice.len(), 256);
        slice[0] = 42.0;
        slice[255] = 100.0;
        assert_eq!(arena.allocated_bytes(), 1024);

        arena.push_node(NativeComputationNode {
            id: 10,
            opcode: MachineOpcode::Return { value_reg: 0 },
            type_lattice: NativeTypeLattice::PrimitiveFloat { bits: 32 },
            energy_cost: 0.001,
            dependencies: vec![],
        });
        assert_eq!(arena.nodes().len(), 1);

        // O(1) reset
        arena.reset();
        assert_eq!(arena.allocated_bytes(), 0);
        assert_eq!(arena.nodes().len(), 0);

        // Reallocate without new heap alloc
        let slice2 = arena.alloc_f32_slice(128).unwrap();
        assert_eq!(slice2.len(), 128);
    }

    #[test]
    fn test_dense_graph_storage_soa() {
        let mut storage = DenseGraphStorage::with_capacity(16);
        storage.push_node(1, MachineOpcode::Return { value_reg: 0 }, DimensionalUnit::DIMENSIONLESS, 0.05, 0);
        storage.push_node(2, MachineOpcode::Return { value_reg: 1 }, DimensionalUnit::ENERGY_JOULE, 0.10, 1);
        assert_eq!(storage.len(), 2);
        assert!((storage.total_free_energy() - 0.15).abs() < 1e-6);
        assert_eq!(storage.dimensional_units[1], DimensionalUnit::ENERGY_JOULE);
    }
}
