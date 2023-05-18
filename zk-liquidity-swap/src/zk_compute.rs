use pbc_zk::{load_sbi, Sbi8, Sbi128};

/// Output variable type
struct AmountAndDirection {
    /// Token amount.
    amount: Sbi128,
    /// The direction of the token swap. Only the lowest bit is used.
    direction: Sbi8,
}

/// Very simple computation that loads the given variable and outputs.
pub fn zk_compute(input_id: i32) -> AmountAndDirection {
    load_sbi::<AmountAndDirection>(input_id)
}
