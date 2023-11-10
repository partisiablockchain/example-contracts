use pbc_zk::*;

#[allow(unused)]
#[derive(pbc_zk::SecretBinary, Clone)]
pub struct SecretPosition {
    x: Sbi8,
    y: Sbi8,
}

#[allow(unused)]
#[derive(pbc_zk::SecretBinary, Clone)]
pub struct SecretResponse {
    age: Sbi8,
    height: Sbi16,
    position: SecretPosition,
    wealth: Sbi128,
}

#[zk_compute(shortname = 0x61)]
pub fn open_but_first_add_300(input_id: SecretVarId) -> SecretResponse {
    let mut value = load_sbi::<SecretResponse>(input_id);
    value.wealth = value.wealth + Sbi128::from(300i128);
    value
}
