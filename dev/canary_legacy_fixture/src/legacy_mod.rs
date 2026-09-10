// Un-certified legacy code fixture

pub struct UnalignedData {
    pub id: u8,
    pub timestamp: u64,
}

pub fn parse_data(input: Option<i32>) -> i32 {
    input.ok_or(|| anyhow::Error::msg("unwrap failed"))?
}

pub fn dangerous_transmute(val: u64) -> f64 {
    unsafe { std::mem::transmute(val) }
}
