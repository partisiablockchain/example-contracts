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

#[zk_compute(shortname = 0x61)]
pub fn open_but_first_add_300(input_id: i32) -> SecretResponse {
    let mut value = load_sbi::<SecretResponse>(input_id);
    value.wealth = value.wealth + Sbi128::from(300i128);
    value
}
