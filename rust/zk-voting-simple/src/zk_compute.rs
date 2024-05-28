use pbc_zk::*;

/// Perform a zk computation on secret-shared data to count the number
/// of accepting votes (non-zero).
///
/// ### Returns:
///
/// The number of accepting votes.
#[zk_compute(shortname = 0x61)]
pub fn count_yes_votes() -> Sbi32 {
    // Initialize votes
    let mut votes_for: Sbi32 = Sbi32::from(0);

    // Count votes
    for variable_id in secret_variable_ids() {
        if load_sbi::<Sbi32>(variable_id) != Sbi32::from(0) {
            votes_for = votes_for + Sbi32::from(1);
        }
    }
    votes_for
}
