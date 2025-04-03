//! Methods for working with rust implementations of signatures
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use k256::sha2::{Digest, Sha256};
use pbc_contract_common::address::{Address, AddressType};

/// Recover the public key from the message and hex encoded signature
pub fn recover_public_key(message: &[u8], signature_hex: &str) -> Option<VerifyingKey> {
    let serialized_signature = hex::decode(signature_hex).ok()?;
    let recovery_id = RecoveryId::try_from(serialized_signature[0]).ok()?;
    let signature = Signature::try_from(&serialized_signature[1..]).ok()?;
    let recovered_key = VerifyingKey::recover_from_msg(message, &signature, recovery_id).ok()?;
    Some(recovered_key)
}

/// Create a pbc address from a k256 public key
pub fn create_address(public_key: &VerifyingKey) -> Address {
    let hashed_public_key = Sha256::digest(public_key.to_encoded_point(false).as_bytes());
    let mut identifier: [u8; 20] = [0; 20];
    identifier.copy_from_slice(&hashed_public_key[12..32]);

    Address {
        address_type: AddressType::Account,
        identifier,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use k256::ecdsa::SigningKey;
    use k256::EncodedPoint;
    use pbc_traits::{ReadRPC, WriteRPC};

    /// Sign a message
    pub fn sign(
        signing_key: &SigningKey,
        message: &[u8],
    ) -> pbc_contract_common::signature::Signature {
        let (signature, rec_id) = signing_key.sign_recoverable(message).unwrap();
        let signature_bytes = signature.to_bytes();

        let mut value_r: [u8; 32] = [0; 32];
        let mut value_s: [u8; 32] = [0; 32];
        value_r.copy_from_slice(&signature_bytes[0..32]);
        value_s.copy_from_slice(&signature_bytes[32..64]);
        pbc_contract_common::signature::Signature {
            recovery_id: rec_id.to_byte(),
            value_r,
            value_s,
        }
    }

    /// Can recover the public key from a signature.
    #[test]
    fn test_verify_key() {
        let message = b"hello";
        let signature = "00a59298dcf8eb1fac238c76ad2fc044647ecc44814887fc7da33ac1cee00c064d606c846471e16efcf62d499a79a17b38e6d0cc673b89c622e5c9702bee582f3b";
        let public_key = recover_public_key(message, signature).unwrap();
        assert_eq!(
            hex::encode(public_key.to_encoded_point(true).as_bytes()),
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5"
        );
    }

    /// Can create a pbc address from a public key.
    #[test]
    fn test_create_address() {
        let public_key = VerifyingKey::from_encoded_point(
            &EncodedPoint::from_bytes(
                hex::decode("02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5")
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let address = create_address(&public_key);
        let vec = hex::decode("00b2e734b5d8da089318d0d2b076c19f59c450855a").unwrap();
        assert_eq!(address, Address::rpc_read_from(&mut vec.as_slice()))
    }

    /// Can sign a message with a secret key and verify the signature with the corresponding
    /// public key.
    #[test]
    fn test_sign_and_verify() {
        let mut secret_key_bytes: [u8; 24] = [0; 24];
        secret_key_bytes[23] = 2;
        let secret_key = SigningKey::from_slice(&secret_key_bytes).unwrap();
        let message = b"hello";

        let signature = sign(&secret_key, message);

        let mut signature_bytes = vec![];
        signature.rpc_write_to(&mut signature_bytes).unwrap();
        let expected_signature = "00a59298dcf8eb1fac238c76ad2fc044647ecc44814887fc7da33ac1cee00c064d606c846471e16efcf62d499a79a17b38e6d0cc673b89c622e5c9702bee582f3b";
        assert_eq!(hex::encode(&signature_bytes), expected_signature);

        let recovered_public_key =
            recover_public_key(message, &hex::encode(&signature_bytes)).unwrap();

        assert_eq!(recovered_public_key, *secret_key.verifying_key());
    }
}
