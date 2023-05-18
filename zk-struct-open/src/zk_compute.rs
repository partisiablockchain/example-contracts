use pbc_zk::{load_sbi, Sbi8, Sbi16, Sbi128};

struct SecretPosition {
    x: Sbi8,
    y: Sbi8,
}

struct SecretResponse {
    age: Sbi8,
    height: Sbi16,
    position: SecretPosition,
    wealth: Sbi128,
}

pub fn zk_compute(input_id: i32) -> SecretResponse {
    let mut value = load_sbi::<SecretResponse>(input_id);
    value.wealth = value.wealth + Sbi128::from(300i128);
    value
}
