#![doc = include_str!("../README.md")]

#[macro_use]
extern crate pbc_contract_codegen;

use crate::SecurityLevelImpl::{Admin, ModeratorA, ModeratorB, User};
use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::sorted_vec_map::SortedVecMap;
use pbc_traits::ReadWriteState;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;
use std::cmp::Ordering;
use std::fmt::Debug;

/// A trait defining the user levels. Must have a lowest level and highest level.
/// The partial order determines the security levels. If a user's level is greater or equal to
/// the protected level of some data. Then the user can modify that data.
pub trait SecurityLevel: PartialOrd + Eq {
    /// The lowest element
    const LOWEST_LEVEL: Self;
    /// The highest element
    const HIGHEST_LEVEL: Self;
}

/// Implementation of a SecurityLevel. This encodes the following security system.
/// ```text
///           Admin
///          /     \
///  ModeratorA   ModeratorB
///          \     /
///           User
/// ```
/// For example, if some data has security level `User`, then every one can modify it. If the data
/// instead has security level `ModeratorA`, then only users with level `ModeratorA` or `Admin`
/// can modify it.
#[derive(PartialEq, Eq, CreateTypeSpec, ReadWriteState, ReadWriteRPC, Debug, Copy, Clone)]
pub enum SecurityLevelImpl {
    /// Admin, highest level
    #[discriminant(0)]
    Admin {},
    /// Moderator A
    #[discriminant(1)]
    ModeratorA {},
    /// Moderator B
    #[discriminant(2)]
    ModeratorB {},
    /// User, lowest level
    #[discriminant(3)]
    User {},
}

impl SecurityLevelImpl {
    const ORDERINGS: [(SecurityLevelImpl, SecurityLevelImpl); 5] = [
        (User {}, ModeratorA {}),
        (User {}, ModeratorB {}),
        (User {}, Admin {}),
        (ModeratorA {}, Admin {}),
        (ModeratorB {}, Admin {}),
    ];
}

impl PartialOrd for SecurityLevelImpl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if SecurityLevelImpl::ORDERINGS
            .iter()
            .any(|(a, b)| (a, b) == (self, other))
        {
            Some(Ordering::Less)
        } else if SecurityLevelImpl::ORDERINGS
            .iter()
            .any(|(a, b)| (a, b) == (other, self))
        {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

impl SecurityLevel for SecurityLevelImpl {
    const LOWEST_LEVEL: Self = User {};
    const HIGHEST_LEVEL: Self = Admin {};
}

/// State of the contract
#[state]
pub struct ContractState {
    access_map: AccessControlMap<SecurityLevelImpl>,
    description: ProtectedData<SecurityLevelImpl, String>,
    currently_held_by: ProtectedData<SecurityLevelImpl, Option<Address>>,
}

/// Data that is protected by the access control system.
#[derive(CreateTypeSpec, ReadWriteState, Debug)]
pub struct ProtectedData<SecurityLevelT: SecurityLevel, E> {
    level: SecurityLevelT,
    data: E,
}

/// Map from account addresses to user levels
#[derive(CreateTypeSpec, ReadWriteState)]
pub struct AccessControlMap<SecurityLevelT: SecurityLevel> {
    map: SortedVecMap<Address, SecurityLevelT>,
}

impl<SecurityLevelT: SecurityLevel + Clone + Debug> AccessControlMap<SecurityLevelT> {
    /// Gets the user's level from the map or the lowest level if they are not present.
    pub fn get_user_level(&self, user: &Address) -> SecurityLevelT {
        self.map
            .get(user)
            .cloned()
            .unwrap_or(SecurityLevelT::LOWEST_LEVEL)
    }

    /// Update a user's level to a new level. The sender of the action can only update users
    /// whose level is below their own, and only update to levels below or equal to their own.
    pub fn update_user_level(
        &mut self,
        sender: &Address,
        user: Address,
        new_level: SecurityLevelT,
    ) {
        let sender_level = self.get_user_level(sender);
        let user_level = self.get_user_level(&user);
        assert!(
            sender_level > user_level,
            "Sender level '{:?}' cannot update user with level '{:?}'",
            sender_level,
            user_level
        );
        assert!(
            sender_level >= new_level,
            "Sender level '{:?}' cannot update user to new level '{:?}'",
            sender_level,
            new_level
        );
        self.map.insert(user, new_level);
    }
}

impl<SecurityLevelT: SecurityLevel + Debug, E> ProtectedData<SecurityLevelT, E> {
    /// Update data. User's level must be greater than or equal to the protected level.
    pub fn update_data(&mut self, user_level: SecurityLevelT, new_data: E) {
        assert!(
            user_level >= self.level,
            "User with level '{:?}' does not have the privilege to update data with level '{:?}'",
            user_level,
            self.level
        );
        self.data = new_data;
    }

    /// Update the level that is protecting the data. Only users with the highest level can do this.
    pub fn update_level(&mut self, user_level: SecurityLevelT, new_level: SecurityLevelT) {
        assert_eq!(
            user_level,
            SecurityLevelT::HIGHEST_LEVEL,
            "Only '{:?}' can update level",
            SecurityLevelT::HIGHEST_LEVEL
        );
        self.level = new_level;
    }
}

/// Initialize a new Generics contract.
///
/// # Arguments
///
/// * `ctx` - the contract context containing information about the sender and the blockchain.
/// * `number` - the initial number.
/// * `car` - the initial car.
///
/// # Returns
///
/// The initial state.
#[init]
pub fn initialize(ctx: ContractContext, description: String) -> ContractState {
    ContractState {
        access_map: AccessControlMap {
            map: SortedVecMap::from([(ctx.sender, SecurityLevelImpl::HIGHEST_LEVEL)]),
        },
        description: ProtectedData {
            level: Admin {},
            data: description,
        },
        currently_held_by: ProtectedData {
            level: User {},
            data: None,
        },
    }
}

/// Update the description of the object. Can only update the description if level is greater or
/// equal to the level of the object (Default Admin).
#[action(shortname = 0x01)]
pub fn update_description(
    ctx: ContractContext,
    mut state: ContractState,
    new_description: String,
) -> ContractState {
    state.description.update_data(
        state.access_map.get_user_level(&ctx.sender),
        new_description,
    );
    state
}

/// Borrow the object. Can only borrow the object if it is not already lent out, and if the
/// borrower's level is greater or equal to the level of the object (Default User).
#[action(shortname = 0x02)]
pub fn borrow_object(ctx: ContractContext, mut state: ContractState) -> ContractState {
    assert!(
        state.currently_held_by.data.is_none(),
        "Object is already lent out"
    );
    state.currently_held_by.update_data(
        state.access_map.get_user_level(&ctx.sender),
        Some(ctx.sender),
    );
    state
}

/// Return the borrowed object to the contract. Only the user who has borrowed the object can
/// return it.
#[action(shortname = 0x03)]
pub fn return_object(ctx: ContractContext, mut state: ContractState) -> ContractState {
    assert!(
        state
            .currently_held_by
            .data
            .is_some_and(|address| address == ctx.sender),
        "Only the user who has borrowed the object can return it"
    );
    state.currently_held_by.data = None;
    state
}

/// Update the string level stored in state. Only Admin can update levels.
#[action(shortname = 0x04)]
pub fn update_description_level(
    ctx: ContractContext,
    mut state: ContractState,
    new_level: SecurityLevelImpl,
) -> ContractState {
    state
        .description
        .update_level(state.access_map.get_user_level(&ctx.sender), new_level);
    state
}

/// Update the level required to borrow. Only Admin can update levels.
#[action(shortname = 0x05)]
pub fn update_borrow_level(
    ctx: ContractContext,
    mut state: ContractState,
    new_level: SecurityLevelImpl,
) -> ContractState {
    state
        .currently_held_by
        .update_level(state.access_map.get_user_level(&ctx.sender), new_level);
    state
}

/// Update a user's level. A user can only update levels of other users, whose level is lower than
/// their own, and only to a new level that is lower or equal to their own.
#[action(shortname = 0x06)]
pub fn update_user_level(
    ctx: ContractContext,
    mut state: ContractState,
    user: Address,
    new_level: SecurityLevelImpl,
) -> ContractState {
    state
        .access_map
        .update_user_level(&ctx.sender, user, new_level);
    state
}
