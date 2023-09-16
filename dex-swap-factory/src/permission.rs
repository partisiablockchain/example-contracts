//! Small utility library to provide permission systems.

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use read_write_rpc_derive::ReadRPC;
use read_write_state_derive::ReadWriteState;

/// Permission enum for modelling permission systems at runtime.
///
/// Intention is to allow contracts creators to specify which addresses are allowed to call
/// specific invocations at initialization.
#[derive(ReadRPC, ReadWriteState, CreateTypeSpec)]
#[repr(C)]
pub enum Permission {
    /// Permission where everybody have the permission.
    #[discriminant(0)]
    Anybody {},

    /// Permission where only those in [`addresses`] have the permission.
    #[discriminant(1)]
    Specific {
        /// Addresses with the permission.
        addresses: Vec<Address>,
    },
}

impl Permission {
    /// Determines whether the given address have this permission.
    ///
    /// ## Parameters
    ///
    /// - `addr`: Address to check permission for.
    ///
    /// ## Return
    ///
    /// Whether the address had this permission.
    pub fn does_address_have_permission(&self, addr: &Address) -> bool {
        match self {
            Permission::Anybody {} => true,
            Permission::Specific { addresses } => addresses.contains(addr),
        }
    }

    /// Asserts that address have this permission.
    ///
    /// Panics when:
    ///
    /// - Address does not have this permission.
    pub fn assert_permission_for(&self, addr: &Address, permission_name: &'static str) {
        assert!(
            self.does_address_have_permission(addr),
            "Address {:?} {:x?} did not have permission \"{}\"",
            addr.address_type,
            addr.identifier,
            permission_name
        );
    }
}
