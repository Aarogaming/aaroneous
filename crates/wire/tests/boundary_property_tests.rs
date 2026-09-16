use proptest::prelude::*;
use wire::*;

proptest! {
    // File-based failure persistence calls `std::env::current_dir()` to
    // resolve where to write regression seeds. Miri's isolation mode blocks
    // that syscall ("getcwd not available when isolation is enabled"),
    // failing every property test here under the Miri CI job regardless of
    // whether the property itself holds. Persistence is a local-dev
    // convenience (replaying a past failing seed); it isn't needed for CI
    // correctness, so disable it instead of touching the isolation policy.
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_cobs_decode_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        let mut out = [0u8; MAX_FRAMED_SIZE];
        let _ = cobs_decode(&bytes, &mut out);
    }

    #[test]
    fn prop_cobs_encode_decode_roundtrip(bytes in prop::collection::vec(any::<u8>(), 0..MAX_PAYLOAD_SIZE)) {
        let mut enc_buf = [0u8; MAX_FRAMED_SIZE];
        if let Ok(enc_len) = cobs_encode(&bytes, &mut enc_buf) {
            let mut dec_buf = [0u8; MAX_FRAMED_SIZE];
            if let Ok(dec_len) = cobs_decode(&enc_buf[..enc_len], &mut dec_buf) {
                prop_assert_eq!(&dec_buf[..dec_len], &bytes[..]);
            }
        }
    }

    #[test]
    fn prop_decode_frame_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..MAX_FRAMED_SIZE)) {
        let _ = decode_frame(&bytes);
    }

    #[test]
    fn prop_telemetry_batch_pod_bytemuck_fuzz(bytes in prop::collection::vec(any::<u8>(), 0..256)) {
        if bytes.len() >= std::mem::size_of::<TelemetryBatchPod>() {
            let slice = &bytes[..std::mem::size_of::<TelemetryBatchPod>()];
            if let Ok(pod) = bytemuck::try_from_bytes::<TelemetryBatchPod>(slice) {
                let _ = pod.active_channel_count();
                let _ = pod.sum_calibrated_f32();
            }
        }
    }
}
