use pbc_zk::*;

/// Perform a zk computation on secret-shared data sum the secret variables.
///
/// ### Returns:
///
/// The sum of the secret variables.
pub fn sum_everything() -> Sbi32 {
    // Initialize state
    let mut sum: Sbi32 = Sbi32::from(0);

    // Sum each variable
    for variable_id in secret_variable_ids() {
        sum = sum + load_sbi::<Sbi32>(variable_id);
    }

    sum
}
