use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand_core::OsRng;
use sha2::{Sha512, Digest};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootKeypair {
    pub seed_hex: String,
    pub public_key_hex: String,
}

pub fn generate_keypair() -> RootKeypair {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let seed_hex = hex::encode(signing_key.to_bytes());
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    RootKeypair {
        seed_hex,
        public_key_hex,
    }
}

pub fn restore_keypair(seed_bytes: &[u8; 32]) -> RootKeypair {
    let signing_key = SigningKey::from_bytes(seed_bytes);
    let verifying_key = signing_key.verifying_key();

    let seed_hex = hex::encode(signing_key.to_bytes());
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    RootKeypair {
        seed_hex,
        public_key_hex,
    }
}

pub fn sign(keypair: &RootKeypair, data: &[u8]) -> String {
    let seed_bytes = hex::decode(&keypair.seed_hex).expect("valid hex");
    let signing_key = SigningKey::from_bytes(
        &seed_bytes.try_into().expect("32 bytes")
    );
    let signature = signing_key.sign(data);
    hex::encode(signature.to_bytes())
}

pub fn verify(public_key_hex: &str, data: &[u8], signature_hex: &str) -> bool {
    let public_key_bytes = hex::decode(public_key_hex).expect("valid hex");
    let signature_bytes = hex::decode(signature_hex).expect("valid hex");

    let verifying_key = match VerifyingKey::from_bytes(
        &public_key_bytes.try_into().expect("32 bytes")
    ) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let sig_array: [u8; 64] = signature_bytes.try_into().expect("64 bytes");
    let signature = Signature::from_bytes(&sig_array);

    verifying_key.verify(data, &signature).is_ok()
}

pub fn derive_ed25519_seed(bip39_seed: &[u8]) -> [u8; 32] {
    let hash = Sha512::digest(bip39_seed);
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&hash[..32]);
    seed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_roundtrip() {
        let kp = generate_keypair();
        let data = b"test data";

        let sig = sign(&kp, data);
        assert!(verify(&kp.public_key_hex, data, &sig));
    }

    #[test]
    fn test_restore_from_seed() {
        let original = generate_keypair();
        let seed_bytes = hex::decode(&original.seed_hex).unwrap();
        let restored = restore_keypair(&seed_bytes.try_into().unwrap());

        assert_eq!(original.public_key_hex, restored.public_key_hex);
        assert_eq!(original.seed_hex, restored.seed_hex);
    }
}
