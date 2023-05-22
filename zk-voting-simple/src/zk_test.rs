//! Test for ZK-computation for secret voting.

mod zk_compute;

#[cfg(test)]
mod tests {
    use crate::zk_compute::zk_compute;
    use pbc_zk::api::*;
    use pbc_zk::*;

    #[test]
    fn zk_compute_zero_one() {
        // assert eval(0,0,1,1,0,0) => 2
        let inputs: Vec<SecretVar> = vec![
            SecretVar {
                metadata: Box::new(1),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(2),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(3),
                value: Box::new(Sbi32::from(1)),
            },
            SecretVar {
                metadata: Box::new(4),
                value: Box::new(Sbi32::from(1)),
            },
            SecretVar {
                metadata: Box::new(5),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(6),
                value: Box::new(Sbi32::from(0)),
            },
        ];

        unsafe {
            set_secrets(inputs);
        }
        let output = zk_compute();
        assert_eq!(output, Sbi32::from(2));
    }

    #[test]
    fn zk_compute_other_values() {
        // assert eval(27,0,1,1,0,123,0,0,0,0,1) => 5
        let inputs: Vec<SecretVar> = vec![
            SecretVar {
                metadata: Box::new(1),
                value: Box::new(Sbi32::from(27)),
            },
            SecretVar {
                metadata: Box::new(2),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(3),
                value: Box::new(Sbi32::from(1)),
            },
            SecretVar {
                metadata: Box::new(4),
                value: Box::new(Sbi32::from(1)),
            },
            SecretVar {
                metadata: Box::new(5),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(6),
                value: Box::new(Sbi32::from(12)),
            },
            SecretVar {
                metadata: Box::new(7),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(8),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(9),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(10),
                value: Box::new(Sbi32::from(0)),
            },
            SecretVar {
                metadata: Box::new(11),
                value: Box::new(Sbi32::from(1)),
            },
        ];

        unsafe {
            set_secrets(inputs);
        }
        let output = zk_compute();
        assert_eq!(output, Sbi32::from(5));
    }
}
