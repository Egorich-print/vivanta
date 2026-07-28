#![allow(dead_code)]

mod identity;
mod state;
mod recovery;
mod environment;
mod dt;
mod hardware;
mod simulator;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("simulate");

    match command {
        "simulate" => {
            let result = simulator::run_continuity_proof();
            std::process::exit(if result.continuity_proven { 0 } else { 1 });
        }
        "init" => {
            let keypair = identity::generate_keypair();
            let seed = recovery::RecoverySeed::generate();
            let seed_phrase = recovery::phrase_to_string(&seed);
            let storage_id = args.get(2).map(|s| s.as_str()).unwrap_or("default-storage");

            let user_data = environment::simulate_user_data();
            let empty_hash = "0".repeat(64);
            let mut env_manifest = environment::create_environment_manifest(
                &keypair.public_key_hex,
                0,
                &empty_hash,
                &empty_hash,
                &user_data,
            );
            env_manifest = environment::sign_environment_manifest(env_manifest, &keypair.seed_hex);
            let env_hash = environment::hash_manifest_json(&env_manifest);

            let mut genesis = state::create_genesis_state(&keypair.public_key_hex, storage_id, &env_hash);
            genesis = state::sign_state_document(genesis, &keypair.seed_hex);

            println!("{}", state::serialize_state(&genesis));
            eprintln!("Recovery seed: {}", seed_phrase);
            eprintln!("Public key:    {}", keypair.public_key_hex);
        }
        "verify" => {
            let mut input = String::new();
            use std::io::Read;
            std::io::stdin().read_to_string(&mut input).expect("stdin");
            let state = state::deserialize_state(input.trim());
            let valid = state::verify_state_document(&state);
            println!("{}", if valid { "VALID" } else { "INVALID" });
            std::process::exit(if valid { 0 } else { 1 });
        }
        "verify-chain" => {
            let mut input = String::new();
            use std::io::Read;
            std::io::stdin().read_to_string(&mut input).expect("stdin");
            let states: Vec<state::StateDocument> = serde_json::from_str(input.trim()).expect("valid JSON array");
            let valid = state::verify_chain(&states);
            println!("{}", if valid { "CHAIN VALID" } else { "CHAIN BROKEN" });
            std::process::exit(if valid { 0 } else { 1 });
        }
        "help" | "--help" | "-h" => {
            println!("Theseus M3 — Incremental Environment Continuity");
            println!();
            println!("Usage:");
            println!("  theseus-m1 simulate        Run the full continuity proof (identity + incremental environment)");
            println!("  theseus-m1 init <storage>   Generate genesis state (standalone)");
            println!("  theseus-m1 verify           Verify a state document (from stdin)");
            println!("  theseus-m1 verify-chain     Verify a state chain (from stdin)");
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: theseus-m1 simulate");
            std::process::exit(1);
        }
    }
}
