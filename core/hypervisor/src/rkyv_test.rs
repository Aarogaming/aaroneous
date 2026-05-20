#[cfg(test)]
mod tests {
    use crate::synapse::{Synapse, SynapsePayload};
    use tempfile::NamedTempFile;

    #[test]
    fn test_rkyv_synapse_zero_copy() {
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path();
        
        let mut synapse = Synapse::new(path, 4096).unwrap();
        
        let payload = SynapsePayload {
            key: "test_task".to_string(),
            data: b"some_binary_data".to_vec(),
            timestamp: 123456789,
        };
        
        // Write payload
        synapse.write_payload(&payload).unwrap();
        
        // Read payload back (deserialization for verification)
        let read_back = synapse.read_payload().unwrap();
        assert_eq!(read_back.key, "test_task");
        assert_eq!(read_back.data, b"some_binary_data");
        assert_eq!(read_back.timestamp, 123456789);
    }
}
