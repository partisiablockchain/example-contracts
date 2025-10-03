//! Perform a zk computation on secret-shared data.
//! Finds the highest bidder and the amount of the second-highest bid
use pbc_zk::*;

/// Computation for finding the highest bidder, and second highest bid amount.
///
/// Works by iterating all variables, and continously keeping track of the highest bid amount,
/// second highest bid amount, and the bidder with the highest amount.
#[zk_compute(shortname = 0x61)]
pub fn run_auction() -> (Sbu32, Sbu32) {
    // Initialize state
    let mut highest_bid_id: Sbu32 = Sbu32::from(0);
    let mut highest_amount: Sbu32 = Sbu32::from(0);
    let mut second_highest_amount: Sbu32 = Sbu32::from(0);

    // Determine max
    for variable_id in secret_variable_ids() {
        if load_sbi::<Sbu32>(variable_id) > highest_amount {
            second_highest_amount = highest_amount;
            highest_amount = load_sbi::<Sbu32>(variable_id);
            highest_bid_id = Sbu32::from(variable_id.raw_id);
        } else if load_sbi::<Sbu32>(variable_id) > second_highest_amount {
            second_highest_amount = load_sbi::<Sbu32>(variable_id);
        }
    }

    // Return highest bidder index, and second highest amount
    (highest_bid_id, second_highest_amount)
}
