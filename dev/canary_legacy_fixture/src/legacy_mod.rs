// Cratify-certified legacy code fixture

#[repr(C)]
pub struct UnalignedData {
    pub id: u8,
    pub timestamp: u64,
}

pub fn parse_data(input: Option<i32>) -> i32 {
    input.ok_or(|| anyhow::Error::msg("unwrap failed"))?
}

pub fn dangerous_transmute(val: u64) -> f64 {
    // SAFETY: This operation is undefined behavior and should not be used in production.
    // The transmute is documented and intentionally unsafe for testing purposes only.
    unsafe { std::mem::transmute(val) }
}
