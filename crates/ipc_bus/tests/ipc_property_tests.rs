use ipc_bus::IpcEvent;
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_ipc_event_bytemuck_fuzz(bytes in prop::collection::vec(any::<u8>(), 0..128)) {
        if bytes.len() >= std::mem::size_of::<IpcEvent>() {
            let slice = &bytes[..std::mem::size_of::<IpcEvent>()];
            if let Ok(evt) = bytemuck::try_from_bytes::<IpcEvent>(slice) {
                let _ = evt.timestamp;
                let _ = evt.component_id;
                let _ = evt.event_type;
                let _ = evt.payload_offset;
                let _ = evt.payload_len;
            }
        }
    }

    #[test]
    fn prop_ipc_event_construct_and_roundtrip(
        ts in any::<u64>(),
        comp in any::<u16>(),
        ev_type in any::<u16>(),
        offset in any::<u32>(),
        len in any::<u32>(),
    ) {
        let evt = IpcEvent::new(ts, comp, ev_type, offset, len);
        let bytes = bytemuck::bytes_of(&evt);
        prop_assert_eq!(bytes.len(), std::mem::size_of::<IpcEvent>());

        let recovered: &IpcEvent = bytemuck::from_bytes(bytes);
        prop_assert_eq!(*recovered, evt);
    }
}
