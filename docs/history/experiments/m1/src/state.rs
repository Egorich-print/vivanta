use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareComponent {
    pub component_class: String,
    pub vendor_id: String,
    pub model_id: String,
    pub serial_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDocument {
    pub system_public_key: String,
    pub sequence_number: u64,
    pub previous_state_hash: String,
    pub genesis_state_hash: String,
    pub hardware_inventory: Vec<HardwareComponent>,
    pub environment_state_hash: String,
    pub migration_reason: String,
    pub timestamp: u64,
    pub content_hash: String,
    pub signature: String,
}

fn hash_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn create_genesis_state(
    public_key_hex: &str,
    storage_identity: &str,
    environment_state_hash: &str,
) -> StateDocument {
    let hardware = vec![HardwareComponent {
        component_class: "storage".to_string(),
        vendor_id: "qemu".to_string(),
        model_id: "virtio-blk".to_string(),
        serial_number: storage_identity.to_string(),
    }];

    let pre_content = format!(
        "{}|{}|{}|{:?}|{}|{}|{}",
        public_key_hex,
        0u64,
        "0000000000000000000000000000000000000000000000000000000000000000",
        hardware,
        environment_state_hash,
        "genesis",
        current_timestamp(),
    );

    let content_hash = hex::encode(hash_bytes(pre_content.as_bytes()));
    let genesis_hash = content_hash.clone();

    StateDocument {
        system_public_key: public_key_hex.to_string(),
        sequence_number: 0,
        previous_state_hash: String::from("0").repeat(64),
        genesis_state_hash: genesis_hash,
        hardware_inventory: hardware,
        environment_state_hash: environment_state_hash.to_string(),
        migration_reason: "genesis".to_string(),
        timestamp: current_timestamp(),
        content_hash: content_hash.clone(),
        signature: String::new(),
    }
}

pub fn sign_state_document(
    mut state: StateDocument,
    seed_hex: &str,
) -> StateDocument {
    let content_to_sign = canonical_string(&state);
    let seed_bytes = hex::decode(seed_hex).expect("valid hex");

    let signing_key = ed25519_dalek::SigningKey::from_bytes(
        &seed_bytes.try_into().expect("32 bytes")
    );
    use ed25519_dalek::Signer;
    let signature = signing_key.sign(content_to_sign.as_bytes());
    state.signature = hex::encode(signature.to_bytes());
    state
}

pub fn verify_state_document(state: &StateDocument) -> bool {
    if state.signature.is_empty() {
        return false;
    }

    let content_to_sign = canonical_string(state);
    use ed25519_dalek::Verifier;

    let public_key_bytes = hex::decode(&state.system_public_key).expect("valid hex");
    let signature_bytes = hex::decode(&state.signature).expect("valid hex");

    let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(
        &public_key_bytes.try_into().expect("32 bytes")
    ) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let sig_array: [u8; 64] = signature_bytes.try_into().expect("64 bytes");
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

    verifying_key.verify(content_to_sign.as_bytes(), &signature).is_ok()
}

pub fn create_migration_state(
    previous_state: &StateDocument,
    public_key_hex: &str,
    new_storage_identity: &str,
    environment_state_hash: &str,
    migration_reason: &str,
) -> StateDocument {
    let previous_hash = hash_state_json(previous_state);
    let hardware = vec![HardwareComponent {
        component_class: "storage".to_string(),
        vendor_id: "qemu".to_string(),
        model_id: "virtio-blk".to_string(),
        serial_number: new_storage_identity.to_string(),
    }];

    let seq = previous_state.sequence_number + 1;

    let pre_content = format!(
        "{}|{}|{}|{:?}|{}|{}|{}",
        public_key_hex,
        seq,
        previous_hash,
        hardware,
        environment_state_hash,
        migration_reason,
        current_timestamp(),
    );

    let content_hash = hex::encode(hash_bytes(pre_content.as_bytes()));

    StateDocument {
        system_public_key: public_key_hex.to_string(),
        sequence_number: seq,
        previous_state_hash: previous_hash,
        genesis_state_hash: previous_state.genesis_state_hash.clone(),
        hardware_inventory: hardware,
        environment_state_hash: environment_state_hash.to_string(),
        migration_reason: migration_reason.to_string(),
        timestamp: current_timestamp(),
        content_hash,
        signature: String::new(),
    }
}

pub fn verify_chain(states: &[StateDocument]) -> bool {
    if states.is_empty() {
        return false;
    }

    let genesis = &states[0];
    if genesis.sequence_number != 0 {
        return false;
    }
    if !verify_state_document(genesis) {
        return false;
    }

    for i in 1..states.len() {
        let state = &states[i];
        if state.sequence_number != i as u64 {
            return false;
        }
        let expected_prev_hash = hash_state_json(&states[i - 1]);
        if state.previous_state_hash != expected_prev_hash {
            return false;
        }
        if state.genesis_state_hash != genesis.content_hash {
            return false;
        }
        if !verify_state_document(state) {
            return false;
        }
    }

    true
}

fn canonical_string(state: &StateDocument) -> String {
    format!(
        "{}|{}|{}|{}|{:?}|{}|{}|{}",
        state.system_public_key,
        state.sequence_number,
        state.previous_state_hash,
        state.genesis_state_hash,
        state.hardware_inventory,
        state.environment_state_hash,
        state.migration_reason,
        state.timestamp,
    )
}

pub fn hash_state_json(state: &StateDocument) -> String {
    let json = serde_json::to_string(state).unwrap();
    hex::encode(hash_bytes(json.as_bytes()))
}

pub fn serialize_state(state: &StateDocument) -> String {
    serde_json::to_string_pretty(state).unwrap()
}

pub fn deserialize_state(json: &str) -> StateDocument {
    serde_json::from_str(json).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    #[test]
    fn test_genesis_state() {
        let env_hash = hex::encode(Sha256::digest(b"test-env"));
        let state = create_genesis_state(
            "abc123",
            "storage-001",
            &env_hash,
        );
        assert_eq!(state.sequence_number, 0);
        assert_eq!(state.hardware_inventory.len(), 1);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = crate::identity::generate_keypair();
        let env_hash = hex::encode(Sha256::digest(b"test-env"));
        let mut state = create_genesis_state(&kp.public_key_hex, "storage-001", &env_hash);
        state = sign_state_document(state, &kp.seed_hex);
        assert!(verify_state_document(&state));
    }

    #[test]
    fn test_chain_verification() {
        let kp = crate::identity::generate_keypair();
        let env_hash0 = hex::encode(Sha256::digest(b"test-env-0"));
        let env_hash1 = hex::encode(Sha256::digest(b"test-env-1"));

        let mut s0 = create_genesis_state(&kp.public_key_hex, "storage-A", &env_hash0);
        s0 = sign_state_document(s0, &kp.seed_hex);

        let mut s1 = create_migration_state(&s0, &kp.public_key_hex, "storage-B", &env_hash1, "storage_replaced");
        s1 = sign_state_document(s1, &kp.seed_hex);

        assert!(verify_chain(&[s0, s1]));
    }
}
