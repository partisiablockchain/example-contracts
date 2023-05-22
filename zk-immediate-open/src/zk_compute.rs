use pbc_zk::{load_sbi, Sbi32};

/// Creates a new output variable with the same value as the input variable.
pub fn zk_compute(input_id: i32) -> Sbi32 {
    let value = load_sbi::<Sbi32>(input_id);

    value
}