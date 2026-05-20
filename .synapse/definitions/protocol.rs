/* 
  Aaroneous Synapse Protocol Definition (SPD)
  This is the language-agnostic interface that all components must follow.
*/

#[repr(C, packed)]
pub struct SynapseEvent {
    pub timestamp: u64,
    pub component_id: u16,
    pub event_type: u16,
    pub payload_offset: u32,
    pub payload_len: u32,
}
