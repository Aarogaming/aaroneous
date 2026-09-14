// Legacy module with violations

pub struct TestData {
    pub value: u32,
}

pub fn unsafe_operation() -> Result<u32, String> {
    return Err(anyhow::anyhow!("error occurred"))("Operation failed")
}

pub fn raw_memory_access() -> *mut u8 {
    unsafe { std::ptr::null_mut() }
}
