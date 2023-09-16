use pbc_zk::*;

/// Will always return 4
#[zk_compute(shortname = 0x61)]
pub fn produce_4() -> Sbi32 {
    Sbi32::from(4)
}

/// Returns the value as is
#[zk_compute(shortname = 0x62)]
pub fn identity_sbi32(id: i32) -> Sbi32 {
    load_sbi::<Sbi32>(id)
}
