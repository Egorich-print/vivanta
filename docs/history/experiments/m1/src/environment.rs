use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareEntry {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentManifest {
    pub system_public_key: String,
    pub sequence_number: u64,
    pub previous_env_hash: String,
    pub state_hash: String,
    pub user_data_hash: String,
    pub configuration_hash: String,
    pub software_inventory: Vec<SoftwareEntry>,
    pub timestamp: u64,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub files: BTreeMap<String, String>,
}

impl UserData {
    pub fn new() -> Self {
        UserData {
            files: BTreeMap::new(),
        }
    }

    pub fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.to_string());
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        for (path, content) in &self.files {
            hasher.update(path.as_bytes());
            hasher.update(b"\x00");
            hasher.update(content.as_bytes());
            hasher.update(b"\x00");
        }
        hex::encode(hasher.finalize())
    }

    pub fn verify_integrity(&self, expected_hash: &str) -> bool {
        self.hash() == expected_hash
    }
}

pub fn create_environment_manifest(
    public_key_hex: &str,
    sequence_number: u64,
    previous_env_hash: &str,
    state_hash: &str,
    user_data: &UserData,
) -> EnvironmentManifest {
    let user_data_hash = user_data.hash();
    let config_hash = hex::encode(Sha256::digest(b"mocked-system-configuration"));

    let software = vec![
        SoftwareEntry {
            name: "theseos-shell".to_string(),
            version: "0.1.0".to_string(),
            hash: hex::encode(Sha256::digest(b"theseos-shell-binary")),
            install_path: "/usr/bin/theseos-shell".to_string(),
        },
        SoftwareEntry {
            name: "theseos-file-manager".to_string(),
            version: "0.1.0".to_string(),
            hash: hex::encode(Sha256::digest(b"theseos-file-manager-binary")),
            install_path: "/usr/bin/theseos-file-manager".to_string(),
        },
    ];

    EnvironmentManifest {
        system_public_key: public_key_hex.to_string(),
        sequence_number,
        previous_env_hash: previous_env_hash.to_string(),
        state_hash: state_hash.to_string(),
        user_data_hash,
        configuration_hash: config_hash,
        software_inventory: software,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        signature: String::new(),
    }
}

pub fn create_environment_update(
    previous_env: &EnvironmentManifest,
    state_hash: &str,
    user_data: &UserData,
) -> EnvironmentManifest {
    create_environment_manifest(
        &previous_env.system_public_key,
        previous_env.sequence_number + 1,
        &hash_manifest_json(previous_env),
        state_hash,
        user_data,
    )
}

pub fn sign_environment_manifest(
    mut manifest: EnvironmentManifest,
    seed_hex: &str,
) -> EnvironmentManifest {
    let content_to_sign = canonical_manifest_string(&manifest);
    let seed_bytes = hex::decode(seed_hex).expect("valid hex");

    let signing_key = ed25519_dalek::SigningKey::from_bytes(
        &seed_bytes.try_into().expect("32 bytes")
    );
    use ed25519_dalek::Signer;
    let signature = signing_key.sign(content_to_sign.as_bytes());
    manifest.signature = hex::encode(signature.to_bytes());
    manifest
}

pub fn verify_environment_manifest(manifest: &EnvironmentManifest) -> bool {
    if manifest.signature.is_empty() {
        return false;
    }

    let content_to_sign = canonical_manifest_string(manifest);
    use ed25519_dalek::Verifier;

    let public_key_bytes = hex::decode(&manifest.system_public_key).expect("valid hex");
    let signature_bytes = hex::decode(&manifest.signature).expect("valid hex");

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

pub fn verify_environment_chain(manifests: &[EnvironmentManifest], data_set: &[UserData]) -> bool {
    if manifests.is_empty() || data_set.is_empty() {
        return false;
    }
    if manifests.len() != data_set.len() {
        return false;
    }

    for i in 0..manifests.len() {
        let manifest = &manifests[i];
        let data = &data_set[i];

        if !verify_environment_manifest(manifest) {
            return false;
        }
        if !data.verify_integrity(&manifest.user_data_hash) {
            return false;
        }
        if i > 0 {
            let expected_prev = hash_manifest_json(&manifests[i - 1]);
            if manifest.previous_env_hash != expected_prev {
                return false;
            }
        }
    }

    true
}

fn canonical_manifest_string(manifest: &EnvironmentManifest) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{:?}|{}",
        manifest.system_public_key,
        manifest.sequence_number,
        manifest.previous_env_hash,
        manifest.state_hash,
        manifest.user_data_hash,
        manifest.configuration_hash,
        manifest.software_inventory,
        manifest.timestamp,
    )
}

pub fn hash_manifest_json(manifest: &EnvironmentManifest) -> String {
    let json = serde_json::to_string(manifest).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn serialize_manifest(manifest: &EnvironmentManifest) -> String {
    serde_json::to_string_pretty(manifest).unwrap()
}

pub fn simulate_user_data() -> UserData {
    let mut data = UserData::new();
    data.add_file(
        "/home/user/documents/report.txt",
        "Theseus OS — Environment Continuity Report\n\nThis file survives storage replacement.\n",
    );
    data.add_file(
        "/home/user/config/settings.json",
        r#"{"theme": "dark", "language": "en", "font_size": 12}"#,
    );
    data.add_file(
        "/home/user/applications/notes.txt",
        "Installed: theseos-shell v0.1.0, theseos-file-manager v0.1.0\n",
    );
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity;

    #[test]
    fn test_user_data_hash_consistency() {
        let data = simulate_user_data();
        let hash1 = data.hash();
        let hash2 = data.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_user_data_modification_detected() {
        let mut data = simulate_user_data();
        let original_hash = data.hash();
        data.add_file("/home/user/config/settings.json", r#"{"theme": "light"}"#);
        assert_ne!(data.hash(), original_hash);
    }

    #[test]
    fn test_environment_manifest_sign_and_verify() {
        let kp = identity::generate_keypair();
        let data = simulate_user_data();
        let state_hash = hex::encode(Sha256::digest(b"mock-state"));

        let mut manifest = create_environment_manifest(
            &kp.public_key_hex, 0, &"0".repeat(64), &state_hash, &data,
        );
        manifest = sign_environment_manifest(manifest, &kp.seed_hex);
        assert!(verify_environment_manifest(&manifest));
    }

    #[test]
    fn test_environment_chain_verification() {
        let kp = identity::generate_keypair();
        let data0 = simulate_user_data();
        let state_hash0 = "0".repeat(64);

        let mut env0 = create_environment_manifest(
            &kp.public_key_hex, 0, &"0".repeat(64), &state_hash0, &data0,
        );
        env0 = sign_environment_manifest(env0, &kp.seed_hex);

        let data1 = simulate_user_data();
        let state_hash1 = hex::encode(Sha256::digest(b"mock-state-1"));

        let mut env1 = create_environment_update(
            &env0, &state_hash1, &data1,
        );
        env1 = sign_environment_manifest(env1, &kp.seed_hex);

        assert!(verify_environment_chain(&[env0, env1], &[data0, data1]));
    }

    #[test]
    fn test_environment_chain_corruption_detected() {
        let kp = identity::generate_keypair();
        let data0 = simulate_user_data();
        let state_hash0 = "0".repeat(64);

        let mut env0 = create_environment_manifest(
            &kp.public_key_hex, 0, &"0".repeat(64), &state_hash0, &data0,
        );
        env0 = sign_environment_manifest(env0, &kp.seed_hex);

        let mut corrupted_data = simulate_user_data();
        corrupted_data.add_file("/home/user/config/settings.json", r#"{"theme": "light"}"#);

        assert!(!verify_environment_chain(&[env0], &[corrupted_data]));
    }

    #[test]
    fn test_incremental_update_chain() {
        let kp = identity::generate_keypair();
        let state_hash = "0".repeat(64);

        let genesis_data = simulate_user_data();
        let mut env0 = create_environment_manifest(
            &kp.public_key_hex, 0, &"0".repeat(64), &state_hash, &genesis_data,
        );
        env0 = sign_environment_manifest(env0, &kp.seed_hex);

        let mut data_after_edit = simulate_user_data();
        data_after_edit.add_file("/home/user/config/settings.json", r#"{"theme": "light"}"#);
        let mut env1 = create_environment_update(&env0, &state_hash, &data_after_edit);
        env1 = sign_environment_manifest(env1, &kp.seed_hex);

        let mut data_after_add = data_after_edit.clone();
        data_after_add.add_file("/home/user/bookmarks.txt", "https://theseus-os.dev\n");
        let mut env2 = create_environment_update(&env1, &state_hash, &data_after_add);
        env2 = sign_environment_manifest(env2, &kp.seed_hex);

        assert!(verify_environment_chain(&[env0.clone(), env1, env2.clone()], &[
            genesis_data,
            data_after_edit,
            data_after_add,
        ]));
        assert_ne!(env0.user_data_hash, env2.user_data_hash);
    }

    #[test]
    fn test_incremental_update_broken_link_detected() {
        let kp = identity::generate_keypair();
        let data = simulate_user_data();
        let state_hash = "0".repeat(64);

        let mut env0 = create_environment_manifest(
            &kp.public_key_hex, 0, &"0".repeat(64), &state_hash, &data,
        );
        env0 = sign_environment_manifest(env0, &kp.seed_hex);

        let mut env1 = create_environment_manifest(
            &kp.public_key_hex, 1, &"x".repeat(64), &state_hash, &data,
        );
        env1 = sign_environment_manifest(env1, &kp.seed_hex);

        assert!(!verify_environment_chain(&[env0, env1], &[data.clone(), data]));
    }
}
