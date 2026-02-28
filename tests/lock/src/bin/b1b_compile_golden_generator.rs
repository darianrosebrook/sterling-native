//! Generator for B1b compile parity golden fixture.
//!
//! Extracts compile inputs from `RomeMiniSearch` in canonical form,
//! runs `compile()`, and writes both inputs and expected outputs to a
//! fixture directory. An external implementation (e.g., v1 Python) can
//! consume the inputs and assert identical outputs.
//!
//! Usage: `b1b_compile_golden_generator <output_dir>`

use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, HashDomain};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: b1b_compile_golden_generator <output_dir>");
        std::process::exit(1);
    }
    let output_dir = std::path::Path::new(&args[1]);

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir).expect("failed to create output directory");
    }

    let world = RomeMiniSearch;

    // Extract compile inputs.
    let payload_bytes = world.encode_payload().expect("encode_payload failed");
    let schema = world.schema_descriptor();
    let concept_registry = world.registry().expect("registry failed");

    // Write inputs.
    std::fs::write(output_dir.join("payload.json"), &payload_bytes)
        .expect("failed to write payload.json");

    let schema_json = canonical_json_bytes(&serde_json::json!({
        "id": schema.id,
        "version": schema.version,
        "hash": schema.hash,
    }))
    .expect("schema canonical JSON failed");
    std::fs::write(output_dir.join("schema_descriptor.json"), &schema_json)
        .expect("failed to write schema_descriptor.json");

    let registry_bytes = concept_registry
        .canonical_bytes()
        .expect("registry canonical_bytes failed");
    std::fs::write(output_dir.join("concept_registry.json"), &registry_bytes)
        .expect("failed to write concept_registry.json");

    // Run compile.
    let result = compile(&payload_bytes, &schema, &concept_registry).expect("compile failed");

    // Write expected outputs.
    std::fs::write(
        output_dir.join("expected_compilation_manifest.json"),
        &result.compilation_manifest,
    )
    .expect("failed to write expected_compilation_manifest.json");

    // Compute the hashes that a reimplementation must match.
    let payload_hash = canonical_hash(HashDomain::CompilationPayload, &payload_bytes);
    let registry_hash = canonical_hash(HashDomain::RegistrySnapshot, &registry_bytes);

    let hashes_json = canonical_json_bytes(&serde_json::json!({
        "evidence_digest": result.evidence_digest.as_str(),
        "identity_digest": result.identity_digest.as_str(),
        "payload_hash": payload_hash.as_str(),
        "registry_epoch": result.registry_descriptor.epoch,
        "registry_hash": registry_hash.as_str(),
        "schema_hash": schema.hash,
    }))
    .expect("hashes canonical JSON failed");
    std::fs::write(output_dir.join("expected_hashes.json"), &hashes_json)
        .expect("failed to write expected_hashes.json");

    println!("identity_digest={}", result.identity_digest.as_str());
    println!("evidence_digest={}", result.evidence_digest.as_str());
    println!("payload_hash={}", payload_hash.as_str());
    println!("registry_hash={}", registry_hash.as_str());
    println!(
        "manifest_bytes={}",
        result.compilation_manifest.len()
    );
    println!("golden written to: {}", output_dir.display());
}
