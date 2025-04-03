use pbc_zk::*;

/// Output variable type
#[derive(pbc_zk::SecretBinary, Clone)]
pub struct RandomnessInput {
    /// Token amount.
    d1: Sbi8,
    d2: Sbi8,
}

/// Perform a zk computation on secret-shared randomness added to make a random dice throw.
///
/// ### Returns:
///
/// The sum of the randomness contributions variables.
#[zk_compute(shortname = 0x61)]
pub fn compute_dice_throw() -> RandomnessInput {
    let mut throw = RandomnessInput {
        d1: Sbi8::from(0),
        d2: Sbi8::from(0),
    };

    for variable_id in secret_variable_ids() {
        let mut raw_contribution: RandomnessInput = load_sbi::<RandomnessInput>(variable_id);

        let d1_reduced = reduce_contribution(raw_contribution.d1);
        let d2_reduced = reduce_contribution(raw_contribution.d2);

        throw.d1 = throw.d1 + d1_reduced;
        throw.d2 = throw.d2 + d2_reduced;
    }

    throw
}

/// Reduce the contribution if it is not between 0 and 5.
fn reduce_contribution(value: Sbi8) -> Sbi8 {
    let reduce = value & Sbi8::from(0b111);
    if reduce >= Sbi8::from(6) {
        reduce - Sbi8::from(6)
    } else {
        reduce
    }
}
