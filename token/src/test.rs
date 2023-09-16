#[cfg(test)]
mod test_contract {
    use pbc_contract_common::address::{Address, AddressType};
    use pbc_contract_common::context::ContractContext;
    use pbc_contract_common::Hash;
    use std::ops::Sub;

    use crate::{
        approve, bulk_transfer, bulk_transfer_from, initialize, transfer, transfer_from,
        TokenState, Transfer,
    };

    const SENDER: Address = Address {
        address_type: AddressType::Account,
        identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    const ALLOWED_SPENDER: Address = Address {
        address_type: AddressType::Account,
        identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    };
    const RECEIVER_1: Address = Address {
        address_type: AddressType::Account,
        identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
    };
    const RECEIVER_2: Address = Address {
        address_type: AddressType::Account,
        identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
    };
    const OWNER: Address = Address {
        address_type: AddressType::Account,
        identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4],
    };
    const CONTRACT_ADDRESS: Address = Address {
        address_type: AddressType::PublicContract,
        identifier: [0u8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9],
    };

    fn create_ctx(sender: Address) -> ContractContext {
        let hash: Hash = Hash {
            bytes: [
                0u8, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1,
            ],
        };
        let ctx: ContractContext = ContractContext {
            contract_address: CONTRACT_ADDRESS,
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
        let ctx = create_ctx(SENDER);
        let state: TokenState = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );

        assert_eq!(1000000, state.total_supply);
        assert_eq!(SENDER, state.owner);
        assert_eq!(0, state.decimals);
        assert_eq!(String::from("HelloToken"), state.name);
        assert_eq!(String::from("H$"), state.symbol);
        assert_eq!(1000000u128, state.balance_of(&SENDER));
        assert!(state.allowed.is_empty());
    }

    #[test]
    pub fn test_transfer() {
        let ctx = create_ctx(SENDER);
        let state: TokenState = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let ctx = create_ctx(SENDER);
        let new_state: TokenState = transfer(ctx, state, RECEIVER_1, 1000);

        assert_eq!(999000u128, new_state.balance_of(&SENDER));
        assert_eq!(1000u128, new_state.balance_of(&RECEIVER_1));
    }

    #[test]
    pub fn test_transfer_same_receiver() {
        let ctx = create_ctx(SENDER);
        let state: TokenState = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let receiver = SENDER;
        let ctx = create_ctx(SENDER);
        let new_state: TokenState = transfer(ctx, state, receiver, 1000);

        assert_eq!(1000000u128, new_state.balance_of(&SENDER));
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_invalid() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 999);
        let ctx = create_ctx(SENDER);
        transfer(ctx, state, RECEIVER_1, 1000);
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_wrong_sender() {
        let ctx = create_ctx(SENDER);
        let state: TokenState = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let wrong_sender = RECEIVER_1;
        transfer(create_ctx(wrong_sender), state, RECEIVER_1, 1000);
    }

    #[test]
    pub fn test_transfer_zero() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 999);
        let ctx = create_ctx(SENDER);
        let new_state: TokenState = transfer(ctx, state, RECEIVER_1, 0);

        assert_eq!(999u128, new_state.balance_of(&SENDER));
        assert_eq!(0u128, new_state.balance_of(&RECEIVER_1));
    }

    #[test]
    pub fn test_bulk_transfer() {
        let ctx = create_ctx(SENDER);
        let state: TokenState = initialize(
            ctx,
            String::from("HelloToken"),
            String::from("H$"),
            0,
            1000000,
        );
        let ctx = create_ctx(SENDER);
        let transfer1 = Transfer {
            to: RECEIVER_1,
            amount: 1000u128,
        };
        let transfer2 = Transfer {
            to: RECEIVER_2,
            amount: 2000u128,
        };
        let transfers = vec![transfer1, transfer2];
        let new_state: TokenState = bulk_transfer(ctx, state, transfers);

        assert_eq!(997000u128, new_state.balance_of(&SENDER));
        assert_eq!(1000u128, new_state.balance_of(&RECEIVER_1));
        assert_eq!(2000u128, new_state.balance_of(&RECEIVER_2));
    }

    #[test]
    #[should_panic]
    pub fn test_bulk_transfer_invalid() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(SENDER);
        let transfer1 = Transfer {
            to: RECEIVER_1,
            amount: 1100u128,
        };
        let transfer2 = Transfer {
            to: RECEIVER_2,
            amount: 700u128,
        };
        let transfers = vec![transfer1, transfer2];
        bulk_transfer(ctx, state, transfers);
    }

    #[test]
    pub fn test_approve() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);

        assert_eq!(0, state.allowed.len());
        let ctx = create_ctx(SENDER);
        let new_state: TokenState = approve(ctx, state, ALLOWED_SPENDER, 100);
        assert_eq!(100u128, new_state.allowance(&SENDER, &ALLOWED_SPENDER));
    }

    #[test]
    pub fn test_approve_overwrite() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);

        assert_eq!(0, state.allowed.len());
        let ctx = create_ctx(SENDER);
        let intermediate_state: TokenState = approve(ctx, state, ALLOWED_SPENDER, 100);
        let ctx = create_ctx(SENDER);
        let new_state: TokenState = approve(ctx, intermediate_state, ALLOWED_SPENDER, 300);
        assert_eq!(300u128, new_state.allowance(&SENDER, &ALLOWED_SPENDER));
    }

    #[test]
    pub fn test_transfer_from() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(SENDER);
        let intermediate_state: TokenState = approve(ctx, state, ALLOWED_SPENDER, 100);
        let ctx = create_ctx(ALLOWED_SPENDER);
        let new_state: TokenState = transfer_from(ctx, intermediate_state, SENDER, RECEIVER_1, 100);

        assert_eq!(0, new_state.allowance(&SENDER, &ALLOWED_SPENDER));

        assert_eq!(900u128, new_state.balance_of(&SENDER));
        assert_eq!(100u128, new_state.balance_of(&RECEIVER_1));
    }

    #[test]
    pub fn test_transfer_from_no_approve() {
        let ctx = create_ctx(OWNER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(ALLOWED_SPENDER);

        let new_state: TokenState = transfer_from(ctx, state, ALLOWED_SPENDER, RECEIVER_1, 0);

        assert_eq!(0, new_state.allowance(&SENDER, &ALLOWED_SPENDER));

        assert_eq!(1000u128, new_state.balance_of(&OWNER));
        assert_eq!(0u128, new_state.balance_of(&RECEIVER_1));
    }

    #[test]
    pub fn test_transfer_from_same_receiver() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(SENDER);
        let receiver = SENDER;
        let intermediate_state: TokenState = approve(ctx, state, ALLOWED_SPENDER, 100);
        let ctx = create_ctx(ALLOWED_SPENDER);
        let new_state: TokenState = transfer_from(ctx, intermediate_state, SENDER, receiver, 100);

        assert_eq!(0, new_state.allowance(&SENDER, &ALLOWED_SPENDER));

        assert_eq!(1000u128, new_state.balance_of(&SENDER));
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_from_not_allowed() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(SENDER);
        let intermediate_state: TokenState = approve(ctx, state, ALLOWED_SPENDER, 100);
        let ctx = create_ctx(ALLOWED_SPENDER);
        let _new_state: TokenState =
            transfer_from(ctx, intermediate_state, SENDER, RECEIVER_1, 101);
    }

    #[test]
    #[should_panic]
    pub fn test_transfer_from_no_funds() {
        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 100);
        let ctx = create_ctx(SENDER);
        let intermediate_state: TokenState = approve(ctx, state, ALLOWED_SPENDER, 1000);
        let ctx = create_ctx(ALLOWED_SPENDER);
        let _new_state: TokenState =
            transfer_from(ctx, intermediate_state, SENDER, RECEIVER_1, 101);
    }

    #[test]
    pub fn test_bulk_transfer_from() {
        let transfer1 = Transfer {
            to: RECEIVER_1,
            amount: 100u128,
        };
        let transfer2 = Transfer {
            to: RECEIVER_2,
            amount: 200u128,
        };
        let transfers = vec![transfer1, transfer2];
        let total_amount_to_transfer = transfers
            .iter()
            .fold(0, |acc, to_and_amount| acc + to_and_amount.amount);

        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(SENDER);
        let intermediate_state: TokenState =
            approve(ctx, state, ALLOWED_SPENDER, total_amount_to_transfer);
        let ctx = create_ctx(ALLOWED_SPENDER);
        let new_state: TokenState = bulk_transfer_from(ctx, intermediate_state, SENDER, transfers);
        assert_eq!(0, new_state.allowance(&SENDER, &ALLOWED_SPENDER));

        assert_eq!(700u128, new_state.balance_of(&SENDER));
        assert_eq!(100u128, new_state.balance_of(&RECEIVER_1));
        assert_eq!(200u128, new_state.balance_of(&RECEIVER_2));
    }

    #[test]
    #[should_panic]
    pub fn test_bulk_transfer_not_allowed() {
        let transfer1 = Transfer {
            to: RECEIVER_1,
            amount: 100u128,
        };
        let transfer2 = Transfer {
            to: RECEIVER_2,
            amount: 200u128,
        };
        let transfers = vec![transfer1, transfer2];
        let total_amount_to_transfer = transfers
            .iter()
            .fold(0, |acc, to_and_amount| acc + to_and_amount.amount);

        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 1000);
        let ctx = create_ctx(SENDER);
        let intermediate_state: TokenState =
            approve(ctx, state, ALLOWED_SPENDER, total_amount_to_transfer.sub(1));
        let ctx = create_ctx(ALLOWED_SPENDER);
        let _new_state: TokenState = bulk_transfer_from(ctx, intermediate_state, SENDER, transfers);
    }

    #[test]
    #[should_panic]
    pub fn test_bulk_transfer_from_no_funds() {
        let transfer1 = Transfer {
            to: RECEIVER_1,
            amount: 100u128,
        };
        let transfer2 = Transfer {
            to: RECEIVER_2,
            amount: 200u128,
        };
        let transfers = vec![transfer1, transfer2];

        let ctx = create_ctx(SENDER);
        let state: TokenState =
            initialize(ctx, String::from("HelloToken"), String::from("H$"), 0, 100);
        let ctx = create_ctx(SENDER);
        let intermediate_state: TokenState = approve(ctx, state, ALLOWED_SPENDER, 1000);
        let ctx = create_ctx(ALLOWED_SPENDER);
        let _new_state: TokenState = bulk_transfer_from(ctx, intermediate_state, SENDER, transfers);
    }
}
