use create_type_spec_derive::CreateTypeSpec;
use hmac::{Hmac, Mac};
use k256::elliptic_curve::PrimeField;
use k256::sha2::Digest;
use k256::{sha2, FieldBytes, Scalar};
use pbc_contract_common::abi::CreateTypeSpec;
use pbc_contract_common::U256;
use pbc_traits::{ReadRPC, ReadWriteState, WriteRPC};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;
use std::ops::Sub;

const NUM_SHARES: usize = 3;

pub trait Sum {
    /// Add a list of values together.
    fn sum(values: &[&Self]) -> Self;
}

impl Sum for U256 {
    fn sum(values: &[&Self]) -> Self {
        let result = values
            .iter()
            .map(|v| scalar_from_u256(v))
            .reduce(|acc, e| acc + e)
            .unwrap();
        u256_from_scalar(result)
    }
}

/// A 1 out of 3 replicated secret share. Each of the shares is shared with one other node.
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Clone, Debug)]
pub struct ReplicatedSecretShare<SecretT>
where
    SecretT: ReadWriteState + ReadRPC + WriteRPC + CreateTypeSpec + Eq,
{
    pub shares: [SecretT; 2],
}

impl<SecretT> ReplicatedSecretShare<SecretT>
where
    SecretT: ReadWriteState + ReadRPC + WriteRPC + CreateTypeSpec + Eq + Sum + Clone,
{
    /// Add a constant to this replicated secret share.
    pub fn add_const(&self, constant: &SecretT, party_index: u32) -> Self {
        if party_index == 0 {
            ReplicatedSecretShare {
                shares: [
                    Sum::sum(&[&self.shares[0], constant]),
                    self.shares[1].clone(),
                ],
            }
        } else if party_index == 1 {
            ReplicatedSecretShare {
                shares: [
                    self.shares[0].clone(),
                    Sum::sum(&[&self.shares[1], constant]),
                ],
            }
        } else {
            self.clone()
        }
    }
}

impl ReplicatedSecretShare<U256> {
    /// Multiply a constant to this replicated secret share.
    pub fn mul_const(&self, constant: &U256) -> Self {
        let left_1 = scalar_from_u256(&self.shares[0]);
        let left_2 = scalar_from_u256(&self.shares[1]);
        let right = scalar_from_u256(constant);
        ReplicatedSecretShare {
            shares: [
                u256_from_scalar(left_1 * right),
                u256_from_scalar(left_2 * right),
            ],
        }
    }
}

impl Sub for &ReplicatedSecretShare<U256> {
    type Output = ReplicatedSecretShare<U256>;

    fn sub(self, rhs: Self) -> Self::Output {
        let left_1 = scalar_from_u256(&self.shares[0]);
        let left_2 = scalar_from_u256(&self.shares[1]);
        let right_1 = scalar_from_u256(&rhs.shares[0]);
        let right_2 = scalar_from_u256(&rhs.shares[1]);
        ReplicatedSecretShare {
            shares: [
                u256_from_scalar(left_1 - right_1),
                u256_from_scalar(left_2 - right_2),
            ],
        }
    }
}

impl Sum for ReplicatedSecretShare<U256> {
    fn sum(values: &[&Self]) -> Self {
        let (share_1, share_2) = values
            .iter()
            .map(|value| {
                (
                    scalar_from_u256(&value.shares[0]),
                    scalar_from_u256(&value.shares[1]),
                )
            })
            .reduce(|(acc_1, acc_2), (elem_1, elem_2)| (acc_1 + elem_1, acc_2 + elem_2))
            .unwrap();
        Self {
            shares: [u256_from_scalar(share_1), u256_from_scalar(share_2)],
        }
    }
}

/// Open a list of three shares. Returns [`None`] if unable to open the shares.
pub fn open_replicated_share<SecretT>(shares: &[&ReplicatedSecretShare<SecretT>]) -> Option<SecretT>
where
    SecretT: ReadWriteState + ReadRPC + WriteRPC + CreateTypeSpec + Eq + Sum,
{
    if shares.len() != NUM_SHARES {
        return None;
    }

    if shares[0].shares[0] != shares[1].shares[1]
        || shares[1].shares[0] != shares[2].shares[1]
        || shares[2].shares[0] != shares[0].shares[1]
    {
        return None;
    }

    Some(Sum::sum(&[
        &shares[0].shares[0],
        &shares[1].shares[0],
        &shares[2].shares[0],
    ]))
}

/// Open a list of shares. If only two shares are available, it tries to open those.
/// Returns [`None`] if unable to open the shares.
pub fn try_open_replicated_share<SecretT>(
    shares: &[Option<&ReplicatedSecretShare<SecretT>>],
) -> Option<SecretT>
where
    SecretT: ReadWriteState + ReadRPC + WriteRPC + CreateTypeSpec + Eq + Sum,
{
    if shares.len() != NUM_SHARES {
        return None;
    }
    let num_defined_shares = shares.iter().filter(|p| p.is_some()).count();
    if num_defined_shares == 3 {
        open_replicated_share(
            &shares
                .iter()
                .map(|op| op.unwrap())
                .collect::<Vec<&ReplicatedSecretShare<SecretT>>>(),
        )
    } else if num_defined_shares == 2 {
        let mut shares = shares.iter().cycle().skip_while(|p| p.is_some()).skip(1);
        open_with_two_shares(
            shares.next().unwrap().unwrap(),
            shares.next().unwrap().unwrap(),
        )
    } else {
        None
    }
}

/// Open two shares. Returns [`None`] if unable to open the shares.
pub fn open_with_two_shares<SecretT>(
    share_1: &ReplicatedSecretShare<SecretT>,
    share_2: &ReplicatedSecretShare<SecretT>,
) -> Option<SecretT>
where
    SecretT: ReadWriteState + ReadRPC + WriteRPC + CreateTypeSpec + Eq + Sum,
{
    if share_1.shares[0] != share_2.shares[1] {
        return None;
    }
    Some(Sum::sum(&[
        &share_1.shares[0],
        &share_1.shares[1],
        &share_2.shares[0],
    ]))
}

/// Convert a [`pbc_contract_common::U256`] into a [`k256::Scalar`].
#[allow(deprecated)]
pub fn scalar_from_u256(value: &U256) -> Scalar {
    Scalar::from_repr(FieldBytes::clone_from_slice(value.as_ref())).unwrap()
}

/// Convert a [`k256::Scalar`] into a [`pbc_contract_common::U256`].
pub fn u256_from_scalar(value: Scalar) -> U256 {
    U256::from_byte_array(value.to_bytes().into())
}

/// Subsession ids used for creating prgs
#[derive(Clone, Copy)]
pub enum SubSessionId {
    GenerateSecretKey = 0,
    PrePrepCheck = 1,
    Prep = 2,
    MulCheckOne = 3,
    MulCheckTwo = 4,
    LinearCombination = 10,
}

/// The prg keys used to generate random shares. Each prg  key is shared with one other
/// node to be able to generate replicated secret shares.
#[derive(ReadWriteState)]
pub(crate) struct ReplicatedSecretSharePrgKeys {
    pub(crate) prgs: [U256; 2],
}

/// The pseudo random generators used to generate random shares.
pub(crate) struct ReplicatedSecretSharePrg {
    prgs: [Prg; 2],
}

impl ReplicatedSecretSharePrg {
    /// Create a new pair of replicated prgs by supplying the prg keys, a session id, and a
    /// sub session id.
    pub fn new(
        prg_keys: &ReplicatedSecretSharePrgKeys,
        session_id: u32,
        sub_session_id: SubSessionId,
    ) -> Self {
        Self {
            prgs: [
                Prg::new(&prg_keys.prgs[0], session_id, sub_session_id),
                Prg::new(&prg_keys.prgs[1], session_id, sub_session_id),
            ],
        }
    }

    /// Generate this node's replicated share of a random value based on the pseudo random
    /// generators.
    pub fn generate_random_share(&mut self) -> ReplicatedSecretShare<U256> {
        ReplicatedSecretShare {
            shares: [
                self.prgs[0].generate_random_value(),
                self.prgs[1].generate_random_value(),
            ],
        }
    }

    /// Generate this node's share of a zero share based on the pseudo random generators.
    /// This generates a non-replicated secret share of the value zero.
    pub fn generate_zero_share(&mut self) -> U256 {
        let share_1 = self.prgs[0].generate_random_value();
        let share_2 = self.prgs[1].generate_random_value();

        let scalar_1 = scalar_from_u256(&share_1);
        let scalar_2 = scalar_from_u256(&share_2);
        let result = scalar_1 - scalar_2;
        u256_from_scalar(result)
    }
}

/// A pseudo random generator for creating random values.
pub(crate) struct Prg {
    /// Seed for this prg
    seed: [u8; 32],
    /// Counter that counts up for each call to this prg to ensure different random values
    counter: u32,
}

impl Prg {
    /// Create a new prg by supplying a prg key, a session id, and a sub session id, all used to
    /// create the seed.
    pub(crate) fn new(key: &U256, session_id: u32, sub_session_id: SubSessionId) -> Self {
        let mut hasher = sha2::Sha256::new();
        hasher.update(key.as_ref());
        hasher.update(session_id.to_le_bytes());
        hasher.update((sub_session_id as u8).to_le_bytes());
        let seed = hasher.finalize();
        Self {
            seed: seed.into(),
            counter: 0,
        }
    }

    /// Increment the counter and generate a random value.
    pub(crate) fn generate_random_value(&mut self) -> U256 {
        self.counter += 1;
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(self.seed.as_ref()).unwrap();
        mac.update(&self.counter.to_le_bytes());
        // We do not take into consideration that the value is greater than the order, as the
        // probability of that happening is less than 2^{-127}
        U256::from_byte_array(mac.finalize().into_bytes().into())
    }
}
