use pbc_zk::*;

/// Creates a new output variable with the same value as the input variable.
#[zk_compute(shortname = 0x61)]
pub fn identity(input_id: SecretVarId) -> Sbi32 {
    load_sbi::<Sbi32>(input_id)
}

test_eq!(identity(SecretVarId::new(1)), 0, [0i32]);
test_eq!(identity(SecretVarId::new(1)), 9, [9i32]);
test_eq!(
    identity(SecretVarId::new(1)),
    -2_147_483_648i32,
    [-2_147_483_648i32]
);
test_eq!(
    identity(SecretVarId::new(1)),
    2_147_483_647i32,
    [2_147_483_647i32]
);
