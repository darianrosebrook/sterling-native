//! Tiny binary that compiles a golden fixture and prints deterministic output.
//!
//! Used by S1-M1-DETERMINISM-CROSSPROC to verify that compilation results
//! are identical across different process environments (cwd, locale, env).
//!
//! Usage: `compile_fixture` `<fixture-path>`
//! Output: three lines, each `key=value`:
//!   `identity_digest=sha256`:...
//!   `evidence_digest=sha256`:...
//!   `evidence_bytes_hex`=...

use sterling_kernel::carrier::bytestate::SchemaDescriptor;
use sterling_kernel::carrier::code32::Code32;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::carrier::registry::RegistryV1;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let fixture_path = args.get(1).expect("usage: compile_fixture <fixture-path>");

    let contents = std::fs::read_to_string(fixture_path)
        .unwrap_or_else(|e| panic!("cannot read fixture at {fixture_path}: {e}"));
    let fixture: serde_json::Value =
        serde_json::from_str(&contents).expect("fixture is valid JSON");

    let allocs: Vec<(Code32, String)> = fixture["registry_allocations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let bytes: Vec<u8> = entry["code32"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
                .collect();
            let code = Code32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let concept_id = entry["concept_id"].as_str().unwrap().to_string();
            (code, concept_id)
        })
        .collect();
    let registry = RegistryV1::new(
        fixture["registry_epoch"].as_str().unwrap().to_string(),
        allocs,
    )
    .unwrap();

    let schema = SchemaDescriptor {
        id: fixture["schema_id"].as_str().unwrap().into(),
        version: fixture["schema_version"].as_str().unwrap().into(),
        hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
    };
    let payload = fixture["canonical_payload"].as_str().unwrap();

    let result = compile(payload.as_bytes(), &schema, &registry).unwrap();

    println!("identity_digest={}", result.identity_digest.as_str());
    println!("evidence_digest={}", result.evidence_digest.as_str());
    println!(
        "evidence_bytes_hex={}",
        hex::encode(result.state.evidence_bytes())
    );
}
