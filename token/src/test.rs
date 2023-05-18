#[cfg(test)]
mod test_contract {
    use pbc_contract_common::address::{Address, AddressType};
    use pbc_contract_common::context::ContractContext;
    use pbc_contract_common::Hash;
    use std::ops::Sub;

    use crate::{
        approve, bulk_transfer, bulk_transfer_from, initialize, transfer, transfer_from, Transfer,
    };

    fn create_ctx(sender: Address) -> ContractContext {
        let hash: Hash = Hash {
            bytes: [
                0u8, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1,
            ],
        };
        let ctx: ContractContext = ContractContext {
            contract_address: Address {
                address_type: AddressType::Account,
                identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            },
            sender,
            block_time: 123,
            block_production_time: 1,
            current_transaction: hash.clone(),
            original_transaction: hash,
        };
        ctx
    }

    #[test]
    pub fn test_initialize() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, events) = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        assert_eq!(0, events.len());
        assert_eq!(1000000, state.total_supply);
        assert_eq!(sender, state.owner);
        assert_eq!(0, state.decimals);
        assert_eq!(String::from("HelloToken"), state.name);
        assert_eq!(String::from("H$"), state.symbol);
        assert_eq!(1, state.balances.len());
        assert_eq!(Some(&1000000u128), state.balances.get(&sender));
        assert!(state.allowed.is_empty());
    }

    #[test]
    pub fn test_transfer() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let ctx = create_ctx(sender);
        let (new_state, events) = transfer(ctx, state, receiver, 1000);
        assert_eq!(0, events.len());
        assert_eq!(2, new_state.balances.len());
        assert_eq!(Some(&999000u128), new_state.balances.get(&sender));
        assert_eq!(Some(&1000u128), new_state.balances.get(&receiver));
    }

    #[test]
    pub fn test_transfer_same_receiver() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let receiver = sender;
        let ctx = create_ctx(sender);
        let (new_state, events) = transfer(ctx, state, receiver, 1000);
        assert_eq!(0, events.len());
        assert_eq!(1, new_state.balances.len());
        assert_eq!(Some(&1000000u128), new_state.balances.get(&sender));
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_invalid() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 999);
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let ctx = create_ctx(sender);
        transfer(ctx, state, receiver, 1000);
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_wrong_sender() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let wrong_sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        };
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let ctx = create_ctx(wrong_sender);
        transfer(ctx, state, receiver, 1000);
    }

    #[test]
    pub fn test_transfer_zero() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 999);
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let ctx = create_ctx(sender);
        let (new_state, events) = transfer(ctx, state, receiver, 0);
        assert_eq!(0, events.len());
        assert_eq!(2, new_state.balances.len());
        assert_eq!(Some(&999u128), new_state.balances.get(&sender));
        assert_eq!(Some(&0u128), new_state.balances.get(&receiver));
    }

    #[test]
    pub fn test_bulk_transfer() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let receiver1 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver2 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        };
        let ctx = create_ctx(sender);
        let transfer1 = Transfer {
            to: receiver1,
            amount: 1000u128,
        };
        let transfer2 = Transfer {
            to: receiver2,
            amount: 2000u128,
        };
        let transfers = vec![transfer1, transfer2];
        let (new_state, events) = bulk_transfer(ctx, state, transfers);
        assert_eq!(0, events.len());
        assert_eq!(3, new_state.balances.len());
        assert_eq!(Some(&997000u128), new_state.balances.get(&sender));
        assert_eq!(Some(&1000u128), new_state.balances.get(&receiver1));
        assert_eq!(Some(&2000u128), new_state.balances.get(&receiver2));
    }

    #[test]
    #[should_panic]
    pub fn test_bulk_transfer_invalid() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let receiver1 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver2 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        };
        let ctx = create_ctx(sender);
        let transfer1 = Transfer {
            to: receiver1,
            amount: 1100u128,
        };
        let transfer2 = Transfer {
            to: receiver2,
            amount: 700u128,
        };
        let transfers = vec![transfer1, transfer2];
        bulk_transfer(ctx, state, transfers);
    }

    #[test]
    pub fn test_approve() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);

        assert_eq!(0, state.allowed.len());
        let ctx = create_ctx(sender);
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let (new_state, events) = approve(ctx, state, allowed_spender, 100);
        assert_eq!(1, new_state.allowed.len());
        assert!(new_state.allowed.contains_key(&sender));
        let allowed_from_sender = new_state.allowed.get(&sender).unwrap();
        assert_eq!(1, allowed_from_sender.len());
        assert!(allowed_from_sender.contains_key(&allowed_spender));
        assert_eq!(&100u128, allowed_from_sender.get(&allowed_spender).unwrap());
    }

    #[test]
    pub fn test_approve_overwrite() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);

        assert_eq!(0, state.allowed.len());
        let ctx = create_ctx(sender);
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let (intermediate_state, _) = approve(ctx, state, allowed_spender, 100);
        let ctx = create_ctx(sender);
        let (new_state, _) = approve(ctx, intermediate_state, allowed_spender, 300);
        assert_eq!(1, new_state.allowed.len());
        assert!(new_state.allowed.contains_key(&sender));
        let allowed_from_sender = new_state.allowed.get(&sender).unwrap();
        assert_eq!(1, allowed_from_sender.len());
        assert!(allowed_from_sender.contains_key(&allowed_spender));
        assert_eq!(&300u128, allowed_from_sender.get(&allowed_spender).unwrap());
    }

    #[test]
    pub fn test_transfer_from() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(sender);
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
        };
        let (intermediate_state, _) = approve(ctx, state, allowed_spender, 100);
        let ctx = create_ctx(allowed_spender);
        let (new_state, events) = transfer_from(ctx, intermediate_state, sender, receiver, 100);
        assert_eq!(1, new_state.allowed.len());
        assert!(new_state.allowed.contains_key(&sender));
        let allowed_from_sender = new_state.allowed.get(&sender).unwrap();
        assert_eq!(1, allowed_from_sender.len());
        assert!(allowed_from_sender.contains_key(&allowed_spender));
        assert_eq!(&0u128, allowed_from_sender.get(&allowed_spender).unwrap());
        assert_eq!(0, events.len());
        assert_eq!(2, new_state.balances.len());
        assert_eq!(Some(&900u128), new_state.balances.get(&sender));
        assert_eq!(Some(&100u128), new_state.balances.get(&receiver));
    }

    #[test]
    pub fn test_transfer_from_no_approve() {
        let owner = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(owner);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        };
        let ctx = create_ctx(allowed_spender);

        let (new_state, _) = transfer_from(ctx, state, allowed_spender, receiver, 0);
        assert_eq!(1, new_state.allowed.len());
        assert!(new_state.allowed.contains_key(&allowed_spender));
        let allowed_from_sender = new_state.allowed.get(&allowed_spender).unwrap();
        assert_eq!(1, allowed_from_sender.len());
        assert!(allowed_from_sender.contains_key(&allowed_spender));
        assert_eq!(&0u128, allowed_from_sender.get(&allowed_spender).unwrap());
        assert_eq!(2, new_state.balances.len());
        assert_eq!(Some(&1000u128), new_state.balances.get(&owner));
        assert_eq!(Some(&0u128), new_state.balances.get(&receiver));
    }

    #[test]
    pub fn test_transfer_from_same_receiver() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(sender);
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver = sender;
        let (intermediate_state, _) = approve(ctx, state, allowed_spender, 100);
        let ctx = create_ctx(allowed_spender);
        let (new_state, events) = transfer_from(ctx, intermediate_state, sender, receiver, 100);
        assert_eq!(1, new_state.allowed.len());
        assert!(new_state.allowed.contains_key(&sender));
        let allowed_from_sender = new_state.allowed.get(&sender).unwrap();
        assert_eq!(1, allowed_from_sender.len());
        assert!(allowed_from_sender.contains_key(&allowed_spender));
        assert_eq!(&0u128, allowed_from_sender.get(&allowed_spender).unwrap());
        assert_eq!(0, events.len());
        assert_eq!(1, new_state.balances.len());
        assert_eq!(Some(&1000u128), new_state.balances.get(&sender));
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_from_not_allowed() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(sender);
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
        };
        let (intermediate_state, _) = approve(ctx, state, allowed_spender, 100);
        let ctx = create_ctx(allowed_spender);
        let (new_state, events) = transfer_from(ctx, intermediate_state, sender, receiver, 101);
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_from_no_funds() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 100);
        let ctx = create_ctx(sender);
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
        };
        let (intermediate_state, _) = approve(ctx, state, allowed_spender, 1000);
        let ctx = create_ctx(allowed_spender);
        let (new_state, events) = transfer_from(ctx, intermediate_state, sender, receiver, 101);
    }

    #[test]
    pub fn test_bulk_transfer_from() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver1 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        };
        let receiver2 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
        };

        let transfer1 = Transfer {
            to: receiver1,
            amount: 100u128,
        };
        let transfer2 = Transfer {
            to: receiver2,
            amount: 200u128,
        };
        let transfers = vec![transfer1, transfer2];
        let total_amount_to_transfer = transfers
            .iter()
            .fold(0, |acc, to_and_amount| acc + to_and_amount.amount);

        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(sender);
        let (intermediate_state, _) =
            approve(ctx, state, allowed_spender, total_amount_to_transfer);
        let ctx = create_ctx(allowed_spender);
        let (new_state, events) = bulk_transfer_from(ctx, intermediate_state, sender, transfers);
        assert_eq!(1, new_state.allowed.len());
        assert!(new_state.allowed.contains_key(&sender));
        let allowed_from_sender = new_state.allowed.get(&sender).unwrap();
        assert_eq!(1, allowed_from_sender.len());
        assert!(allowed_from_sender.contains_key(&allowed_spender));
        assert_eq!(&0u128, allowed_from_sender.get(&allowed_spender).unwrap());
        assert_eq!(0, events.len());
        assert_eq!(3, new_state.balances.len());
        assert_eq!(Some(&700u128), new_state.balances.get(&sender));
        assert_eq!(Some(&100u128), new_state.balances.get(&receiver1));
        assert_eq!(Some(&200u128), new_state.balances.get(&receiver2));
    }

    #[test]
    #[should_panic]
    pub fn test_bulk_transfer_not_allowed() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver1 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        };
        let receiver2 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
        };

        let transfer1 = Transfer {
            to: receiver1,
            amount: 100u128,
        };
        let transfer2 = Transfer {
            to: receiver2,
            amount: 200u128,
        };
        let transfers = vec![transfer1, transfer2];
        let total_amount_to_transfer = transfers
            .iter()
            .fold(0, |acc, to_and_amount| acc + to_and_amount.amount);

        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(sender);
        let (intermediate_state, _) =
            approve(ctx, state, allowed_spender, total_amount_to_transfer.sub(1));
        let ctx = create_ctx(allowed_spender);
        let (new_state, events) = bulk_transfer_from(ctx, intermediate_state, sender, transfers);
    }

    #[test]
    #[should_panic]
    pub fn test_bulk_transfer_from_no_funds() {
        let sender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let allowed_spender = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        };
        let receiver1 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
        };
        let receiver2 = Address {
            address_type: AddressType::Account,
            identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
        };

        let transfer1 = Transfer {
            to: receiver1,
            amount: 100u128,
        };
        let transfer2 = Transfer {
            to: receiver2,
            amount: 200u128,
        };
        let transfers = vec![transfer1, transfer2];
        let total_amount_to_transfer = transfers
            .iter()
            .fold(0, |acc, to_and_amount| acc + to_and_amount.amount);

        let ctx = create_ctx(sender);
        let (state, _) = initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 100);
        let ctx = create_ctx(sender);
        let (intermediate_state, _) = approve(ctx, state, allowed_spender, 1000);
        let ctx = create_ctx(allowed_spender);
        let (new_state, events) = bulk_transfer_from(ctx, intermediate_state, sender, transfers);
    }
}
