use bytemuck::{Pod, Zeroable};

/// Static telemetry descriptor for zero-allocation microkernel component inspection.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ComponentMetadataPod {
    pub component_id: u32,
    pub tier: u8,
    pub methodology: u8,
    pub _pad0: u16,
    pub worst_case_exec_time_ns: u64,
    pub max_memory_footprint_bytes: u64,
    pub version_packed: u32,
    pub capabilities: u32,
    pub name: [u8; 32],
}

/// Static telemetry descriptor for zero-allocation microkernel channel topology inspection.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct ChannelTopologyPod {
    pub channel_id: u32,
    pub src_component: u32,
    pub dst_component: u32,
    pub capacity_events: u32,
    pub ring_buffer_bytes: u64,
    pub name: [u8; 32],
    pub _pad0: [u8; 8],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_metadata_size() {
        assert_eq!(size_of::<ComponentMetadataPod>(), 64);
    }

    #[test]
    fn test_channel_topology_size() {
        assert_eq!(size_of::<ChannelTopologyPod>(), 64);
    }

    #[test]
    fn test_component_metadata_bytemuck_roundtrip() {
        let mut name_buf = [0u8; 32];
        name_buf[..14].copy_from_slice(b"test_component");

        let original = ComponentMetadataPod {
            component_id: 0x12345678,
            tier: 2,
            methodology: 1,
            _pad0: 0x1234,
            worst_case_exec_time_ns: 1000000,
            max_memory_footprint_bytes: 4096,
            version_packed: 0x01020304,
            capabilities: 0x00000001,
            name: name_buf,
        };

        let bytes = bytemuck::bytes_of(&original);
        assert_eq!(bytes.len(), 64);
        let decoded: &ComponentMetadataPod = bytemuck::from_bytes(bytes);
        assert_eq!(*decoded, original);
    }

    #[test]
    fn test_channel_topology_bytemuck_roundtrip() {
        let mut name_buf = [0u8; 32];
        name_buf[..12].copy_from_slice(b"test_channel");

        let original = ChannelTopologyPod {
            channel_id: 0xABCDEF00,
            src_component: 0x11111111,
            dst_component: 0x22222222,
            capacity_events: 1000,
            ring_buffer_bytes: 8192,
            name: name_buf,
            _pad0: [0u8; 8],
        };

        let bytes = bytemuck::bytes_of(&original);
        assert_eq!(bytes.len(), 64);
        let decoded: &ChannelTopologyPod = bytemuck::from_bytes(bytes);
        assert_eq!(*decoded, original);
    }

    #[test]
    fn test_component_metadata_offsets() {
        assert_eq!(std::mem::offset_of!(ComponentMetadataPod, component_id), 0);
        assert_eq!(std::mem::offset_of!(ComponentMetadataPod, tier), 4);
        assert_eq!(std::mem::offset_of!(ComponentMetadataPod, methodology), 5);
        assert_eq!(std::mem::offset_of!(ComponentMetadataPod, _pad0), 6);
        assert_eq!(
            std::mem::offset_of!(ComponentMetadataPod, worst_case_exec_time_ns),
            8
        );
        assert_eq!(
            std::mem::offset_of!(ComponentMetadataPod, max_memory_footprint_bytes),
            16
        );
        assert_eq!(
            std::mem::offset_of!(ComponentMetadataPod, version_packed),
            24
        );
        assert_eq!(std::mem::offset_of!(ComponentMetadataPod, capabilities), 28);
        assert_eq!(std::mem::offset_of!(ComponentMetadataPod, name), 32);
    }

    #[test]
    fn test_channel_topology_offsets() {
        assert_eq!(std::mem::offset_of!(ChannelTopologyPod, channel_id), 0);
        assert_eq!(std::mem::offset_of!(ChannelTopologyPod, src_component), 4);
        assert_eq!(std::mem::offset_of!(ChannelTopologyPod, dst_component), 8);
        assert_eq!(
            std::mem::offset_of!(ChannelTopologyPod, capacity_events),
            12
        );
        assert_eq!(
            std::mem::offset_of!(ChannelTopologyPod, ring_buffer_bytes),
            16
        );
        assert_eq!(std::mem::offset_of!(ChannelTopologyPod, name), 24);
        assert_eq!(std::mem::offset_of!(ChannelTopologyPod, _pad0), 56);
    }
}
