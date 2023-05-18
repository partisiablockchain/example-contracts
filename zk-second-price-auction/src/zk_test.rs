//! Test for ZK-computation for Second-price auction.

mod zk_compute;

#[cfg(test)]
mod tests {
    use crate::zk_compute::zk_compute;
    use pbc_zk::api::*;
    use pbc_zk::*;

    // assert eval(1500, 1900, 2100, 2000, 2050, 1975) => (3, 2050)
    #[test]
    fn second_price_auction_test1() {
        let bids: Vec<SecretVar> = vec![
            SecretVar {
                metadata: Box::new(1),
                value: Box::new(Sbi32::from(1500)),
            },
            SecretVar {
                metadata: Box::new(2),
                value: Box::new(Sbi32::from(1900)),
            },
            SecretVar {
                metadata: Box::new(3),
                value: Box::new(Sbi32::from(2100)),
            },
            SecretVar {
                metadata: Box::new(4),
                value: Box::new(Sbi32::from(2000)),
            },
            SecretVar {
                metadata: Box::new(5),
                value: Box::new(Sbi32::from(2050)),
            },
            SecretVar {
                metadata: Box::new(6),
                value: Box::new(Sbi32::from(1975)),
            },
        ];

        unsafe {
            set_secrets(bids);
        }
        let (highest_bidder, winning_amount) = zk_compute();
        assert_eq!(highest_bidder, Sbi32::from(3));
        assert_eq!(winning_amount, Sbi32::from(2050));
    }

    // assert eval(1,2,3,4,5,6,7,8,9,8,7,6,5,4,3,2,1) => (9, 8)
    #[test]
    fn second_price_auction_test2() {
        let bids: Vec<SecretVar> = vec![
            SecretVar {
                metadata: Box::new(1),
                value: Box::new(Sbi32::from(1)),
            },
            SecretVar {
                metadata: Box::new(2),
                value: Box::new(Sbi32::from(2)),
            },
            SecretVar {
                metadata: Box::new(3),
                value: Box::new(Sbi32::from(3)),
            },
            SecretVar {
                metadata: Box::new(4),
                value: Box::new(Sbi32::from(4)),
            },
            SecretVar {
                metadata: Box::new(5),
                value: Box::new(Sbi32::from(5)),
            },
            SecretVar {
                metadata: Box::new(6),
                value: Box::new(Sbi32::from(6)),
            },
            SecretVar {
                metadata: Box::new(7),
                value: Box::new(Sbi32::from(7)),
            },
            SecretVar {
                metadata: Box::new(8),
                value: Box::new(Sbi32::from(8)),
            },
            SecretVar {
                metadata: Box::new(9),
                value: Box::new(Sbi32::from(9)),
            },
            SecretVar {
                metadata: Box::new(8),
                value: Box::new(Sbi32::from(8)),
            },
            SecretVar {
                metadata: Box::new(7),
                value: Box::new(Sbi32::from(7)),
            },
            SecretVar {
                metadata: Box::new(6),
                value: Box::new(Sbi32::from(6)),
            },
            SecretVar {
                metadata: Box::new(5),
                value: Box::new(Sbi32::from(5)),
            },
            SecretVar {
                metadata: Box::new(4),
                value: Box::new(Sbi32::from(4)),
            },
            SecretVar {
                metadata: Box::new(3),
                value: Box::new(Sbi32::from(3)),
            },
            SecretVar {
                metadata: Box::new(2),
                value: Box::new(Sbi32::from(2)),
            },
            SecretVar {
                metadata: Box::new(1),
                value: Box::new(Sbi32::from(1)),
            },
        ];

        unsafe {
            set_secrets(bids);
        }
        let (highest_bidder, winning_amount) = zk_compute();
        assert_eq!(highest_bidder, Sbi32::from(9));
        assert_eq!(winning_amount, Sbi32::from(8));
    }
}
