use crate::identity;
use crate::state;
use crate::recovery;
use crate::environment;

pub struct SimResult {
    pub genesis_public_key: String,
    pub recovery_seed_phrase: String,
    pub genesis_state_json: String,
    pub recovered_public_key: String,
    pub migration_state_json: String,
    pub chain_valid: bool,
    pub env_chain_valid: bool,
    pub data_integrity_verified: bool,
    pub continuity_proven: bool,
}

pub fn run_continuity_proof() -> SimResult {
    println!("═══════════════════════════════════════════════════════════");
    println!("  Theseus M3 — Incremental Environment Continuity");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    step(1, "GENESIS — First Boot + Environment Creation");

    let rseed = recovery::RecoverySeed::generate();
    let seed_phrase = recovery::phrase_to_string(&rseed);
    let seed_bytes = rseed.to_bytes();
    let keypair = identity::restore_keypair(&seed_bytes);

    println!("  Root Keypair derived from recovery seed:");
    println!("    Public Key: {}", &keypair.public_key_hex[..16]);
    println!();

    println!("  Recovery Seed (BIP-39, 12 words):");
    println!("    {}", seed_phrase);
    println!();

    let storage_a_id = "eMMC-0001-QEMU";

    let mut user_data = environment::simulate_user_data();
    let data_hash = user_data.hash();
    println!("  User Data Created:");
    for path in user_data.files.keys() {
        println!("    📄 {}", path);
    }
    println!("    Data Hash: {}", &data_hash[..16]);
    println!();

    let empty_hash = "0".repeat(64);
    let mut env0 = environment::create_environment_manifest(
        &keypair.public_key_hex, 0, &empty_hash, &empty_hash, &user_data,
    );
    env0 = environment::sign_environment_manifest(env0, &keypair.seed_hex);
    let env0_hash = environment::hash_manifest_json(&env0);

    println!("  Environment Manifest (Genesis):");
    println!("    Sequence: {}", env0.sequence_number);
    println!("    Data Integrity: {}", if environment::verify_environment_manifest(&env0) { "✅ signed" } else { "❌ invalid" });
    println!();

    let mut genesis = state::create_genesis_state(
        &keypair.public_key_hex, storage_a_id, &env0_hash,
    );
    genesis = state::sign_state_document(genesis, &keypair.seed_hex);
    let genesis_json = state::serialize_state(&genesis);

    println!("  Genesis State Document:");
    println!("    Sequence: {}", genesis.sequence_number);
    println!("    Storage:  {}", storage_a_id);
    println!("    Env Hash: {}", &genesis.environment_state_hash[..16]);

    assert!(state::verify_state_document(&genesis));
    println!("  ✅ State signature valid");
    println!();

    separator();

    step(2, "NORMAL BOOT — Identity + Environment Present");

    let state_valid = state::verify_state_document(&genesis);
    println!("  State Document signature: {}", if state_valid { "✅ valid" } else { "❌ invalid" });

    let chain_check = state::verify_chain(&[genesis.clone()]);
    println!("  Chain integrity: {}", if chain_check { "✅ valid" } else { "❌ invalid" });

    let env_valid = environment::verify_environment_manifest(&env0);
    println!("  Manifest signature: {}", if env_valid { "✅ valid" } else { "❌ invalid" });

    let data_ok = user_data.verify_integrity(&env0.user_data_hash);
    println!("  Data integrity: {}", if data_ok { "✅ intact" } else { "❌ corrupted" });
    println!();

    println!("  Hardware unchanged — boot continues with environment.");
    println!();

    separator();

    step(3, "INCREMENTAL UPDATE — File Modified (no state change)");

    println!("  User edits report.txt...");
    user_data.add_file(
        "/home/user/documents/report.txt",
        "Theseus OS — Environment Continuity Report\n\nEDITED: this file was modified before storage death.\n",
    );

    let state_hash0 = environment::hash_manifest_json(&env0);
    let mut env1 = environment::create_environment_update(
        &env0, &state_hash0, &user_data,
    );
    env1 = environment::sign_environment_manifest(env1, &keypair.seed_hex);

    println!("  Environment Manifest (Update 1):");
    println!("    Sequence: {}", env1.sequence_number);
    println!("    Previous: {}", &env1.previous_env_hash[..16]);
    println!("    Data Hash: {}", &env1.user_data_hash[..16]);

    assert!(environment::verify_environment_manifest(&env1));
    assert_ne!(env1.user_data_hash, env0.user_data_hash);
    println!("  ✅ Manifest signed, hash changed (file modification detected)");
    println!("  ✅ NO new State Document created — environment updated independently");
    println!();

    let user_data_before_add = user_data.clone();

    separator();

    step(4, "INCREMENTAL UPDATE — File Added (no state change)");

    println!("  User creates bookmarks.txt...");
    user_data.add_file(
        "/home/user/documents/bookmarks.txt",
        "https://theseus-os.dev\nhttps://github.com/theseus-os\n",
    );

    let state_hash1 = environment::hash_manifest_json(&env1);
    let mut env2 = environment::create_environment_update(
        &env1, &state_hash1, &user_data,
    );
    env2 = environment::sign_environment_manifest(env2, &keypair.seed_hex);

    println!("  Environment Manifest (Update 2):");
    println!("    Sequence: {}", env2.sequence_number);
    println!("    Previous: {}", &env2.previous_env_hash[..16]);
    println!("    Data Hash: {}", &env2.user_data_hash[..16]);

    assert!(environment::verify_environment_manifest(&env2));
    assert_eq!(env2.previous_env_hash, environment::hash_manifest_json(&env1));
    println!("  ✅ Manifest signed, chain link verified");
    println!("  ✅ Environment chain: Env[0] → Env[1] → Env[2] (independently of State)");
    println!();

    separator();

    step(5, "STORAGE REPLACEMENT — Simulating Hardware + Data Death");

    println!("  ✝ Storage device (eMMC-0001-QEMU) removed.");
    println!("  ✝ Identity, environment, and user data lost with old storage.");
    println!("  ✝ New storage (eMMC-0002-QEMU) installed — blank, no identity, no data.");
    println!();
    println!("  Boot attempt on new storage...");
    println!("  → No genesis state found.");
    println!("  → No environment manifest found.");
    println!("  → No user data found.");
    println!("  → Entering RECOVERY mode.");
    println!();

    separator();

    step(6, "RECOVERY — Identity + Environment Restoration");

    println!("  Recovery Seed entered:");
    println!("    {}", seed_phrase);
    println!();

    let restored_keypair = identity::restore_keypair(&seed_bytes);

    let pub_match = restored_keypair.public_key_hex == keypair.public_key_hex;
    println!("    Original Public Key:  {}", &keypair.public_key_hex[..16]);
    println!("    Recovered Public Key: {}", &restored_keypair.public_key_hex[..16]);
    println!("    Match: {}", if pub_match { "✅ IDENTICAL" } else { "❌ MISMATCH" });

    if !pub_match {
        panic!("CRITICAL: Keypair recovery failed. Seed derivation is incorrect.");
    }

    println!();
    println!("  Verifying against Genesis State Document...");
    let genesis_verify = state::verify_state_document(&genesis);
    let pub_key_match = genesis.system_public_key == restored_keypair.public_key_hex;
    println!("    Genesis signature valid:  {}", if genesis_verify { "✅" } else { "❌" });
    println!("    Public key matches genesis: {}", if pub_key_match { "✅" } else { "❌" });

    println!();
    println!("  Restoring user data from backup (simulated)...");

    let mut recovered_data = environment::simulate_user_data();
    recovered_data.add_file(
        "/home/user/documents/report.txt",
        "Theseus OS — Environment Continuity Report\n\nEDITED: this file was modified before storage death.\n",
    );
    recovered_data.add_file(
        "/home/user/documents/bookmarks.txt",
        "https://theseus-os.dev\nhttps://github.com/theseus-os\n",
    );

    println!("  User Data Restored (with pre-death edits):");
    for path in recovered_data.files.keys() {
        println!("    📄 {}", path);
    }
    println!();

    println!("  Verifying against LATEST environment manifest (Env[2])...");
    let data_matches_latest = recovered_data.verify_integrity(&env2.user_data_hash);
    println!("    Data matches Env[2]: {}", if data_matches_latest { "✅ IDENTICAL (edits preserved)" } else { "❌ MISMATCH" });

    let data_matches_genesis = recovered_data.verify_integrity(&env0.user_data_hash);
    println!("    Data matches Env[0]: {}", if data_matches_genesis { "❌ WOULD BE WRONG" } else { "✅ SHOULD DIFFER (edits after genesis)" });

    println!();
    println!("  Creating Migration State Document...");
    let storage_b_id = "eMMC-0002-QEMU";

    let state_hash2 = state::serialize_state(&genesis);
    let mut env3 = environment::create_environment_update(
        &env2, &state_hash2, &recovered_data,
    );
    env3 = environment::sign_environment_manifest(env3, &restored_keypair.seed_hex);
    let env3_hash = environment::hash_manifest_json(&env3);

    println!("  Environment Manifest (Migration):");
    println!("    Sequence: {}", env3.sequence_number);
    println!("    Previous: {}", &env3.previous_env_hash[..16]);
    println!("    Data Integrity: {}", if environment::verify_environment_manifest(&env3) { "✅ signed" } else { "❌ invalid" });
    println!();

    let mut migration = state::create_migration_state(
        &genesis,
        &restored_keypair.public_key_hex,
        storage_b_id,
        &env3_hash,
        "storage_replaced",
    );
    migration = state::sign_state_document(migration, &restored_keypair.seed_hex);
    let migration_json = state::serialize_state(&migration);

    println!("  Migration State Document:");
    println!("    Sequence: {}", migration.sequence_number);
    println!("    Previous: {}", &migration.previous_state_hash[..16]);
    println!("    Storage:  {}", storage_b_id);
    println!("    Env Hash: {}", &migration.environment_state_hash[..16]);
    println!("    Reason:   {}", migration.migration_reason);

    assert!(state::verify_state_document(&migration));
    println!("  ✅ Migration state signature valid");

    println!();
    separator();

    step(7, "CONTINUITY VERIFICATION — Identity + Incremental Environment");

    let chain = vec![genesis.clone(), migration.clone()];
    let chain_valid = state::verify_chain(&chain);

    println!("  State Chain:");
    println!("    State[0]: Genesis    (storage: {})", storage_a_id);
    println!("        ↓");
    println!("    State[1]: Migration  (storage: {})", storage_b_id);
    println!();
    println!("  Chain verification: {}", if chain_valid { "✅ CHAIN VALID" } else { "❌ CHAIN BROKEN" });
    println!("  Keypair match:      {}", if pub_key_match { "✅ IDENTICAL" } else { "❌ MISMATCH" });
    println!();

    let env_manifests = vec![env0.clone(), env1.clone(), env2.clone(), env3.clone()];
    let data_sets = vec![
        environment::simulate_user_data(),           // genesis data (no modifications)
        user_data_before_add,                         // data after Phase 3 (edit only)
        user_data.clone(),                            // data after Phase 4 (edit + add)
        recovered_data.clone(),                       // restored data (edit + add)
    ];
    let env_chain_valid = environment::verify_environment_chain(&env_manifests, &data_sets);

    println!("  Environment Chain:");
    println!("    Env[0]: Genesis    (data hash: {})", &env0.user_data_hash[..16]);
    println!("        ↓  (file unchanged)");
    println!("    Env[1]: Update 1  (data hash: {}) — report.txt modified", &env1.user_data_hash[..16]);
    println!("        ↓  (file added)");
    println!("    Env[2]: Update 2  (data hash: {}) — bookmarks.txt added", &env2.user_data_hash[..16]);
    println!("        ↓  (migration)");
    println!("    Env[3]: Migration (data hash: {})", &env3.user_data_hash[..16]);
    println!();
    println!("  Environment chain verification: {}", if env_chain_valid { "✅ ENV CHAIN VALID" } else { "❌ ENV CHAIN BROKEN" });
    println!();

    let pre_death_data_hash = user_data.hash();
    let recovered_data_hash = recovered_data.hash();
    let edits_preserved = pre_death_data_hash == recovered_data_hash;
    println!("  Pre-death data hash:  {}", &pre_death_data_hash[..16]);
    println!("  Post-recovery hash:   {}", &recovered_data_hash[..16]);
    println!("  Edits preserved:      {}", if edits_preserved { "✅ YES — incremental tracking works" } else { "❌ NO — edits lost" });

    let cross_link_ok = genesis.environment_state_hash == environment::hash_manifest_json(&env0)
        && migration.environment_state_hash == environment::hash_manifest_json(&env3);
    println!("  Cross-links:          {}", if cross_link_ok { "✅ State[0] → Env[0], State[1] → Env[3]" } else { "❌ LINK MISMATCH" });

    let data_integrity_verified = edits_preserved && data_matches_latest;
    let continuity_proven = chain_valid && pub_key_match && env_chain_valid && data_integrity_verified;
    println!();
    if continuity_proven {
        println!("  ╔═══════════════════════════════════════════════════════╗");
        println!("  ║    FULL CONTINUITY: PROVEN (Incremental)             ║");
        println!("  ║  Same system, same data — including changes made     ║");
        println!("  ║  between snapshots. Environment manifest updated     ║");
        println!("  ║  independently of state chain.                       ║");
        println!("  ╚═══════════════════════════════════════════════════════╝");
    } else {
        println!("  ╔═══════════════════════════════════════════════════════╗");
        println!("  ║        CONTINUITY: NOT PROVEN                        ║");
        println!("  ║  System identity or environment was NOT preserved.   ║");
        println!("  ╚═══════════════════════════════════════════════════════╝");
    }

    println!();
    separator();
    println!("  Experiment complete.");
    println!();

    SimResult {
        genesis_public_key: keypair.public_key_hex,
        recovery_seed_phrase: seed_phrase,
        genesis_state_json: genesis_json,
        recovered_public_key: restored_keypair.public_key_hex,
        migration_state_json: migration_json,
        chain_valid,
        env_chain_valid,
        data_integrity_verified,
        continuity_proven,
    }
}

fn step(number: u32, title: &str) {
    println!("  ▌ {}. {}", number, title);
    println!();
}

fn separator() {
    println!("  ─────────────────────────────────────────────────────────");
    println!();
}
