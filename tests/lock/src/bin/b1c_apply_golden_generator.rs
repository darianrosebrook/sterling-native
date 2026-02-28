//! Generator for B1c apply parity golden fixture.
//!
//! Compiles the `RomeMiniSearch` initial state (same as B1b), then applies
//! a single deterministic `SET_SLOT` operation and writes the inputs and
//! expected outputs to a fixture directory. An external implementation can
//! consume the inputs and assert identical outputs.
//!
//! Usage: `b1c_apply_golden_generator <output_dir>`

use sterling_harness::contract::WorldHarnessV1;
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_kernel::carrier::compile::compile;
use sterling_kernel::operators::apply::{apply, set_slot_args, OP_SET_SLOT};
use sterling_kernel::operators::operator_registry::kernel_operator_registry;
use sterling_kernel::proof::canon::canonical_json_bytes;
use sterling_kernel::proof::hash::{canonical_hash, HashDomain};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: b1c_apply_golden_generator <output_dir>");
        std::process::exit(1);
    }
    let output_dir = std::path::Path::new(&args[1]);

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir).expect("failed to create output directory");
    }

    let world = RomeMiniSearch;

    // Step 1: Compile initial state (same as B1b).
    let payload_bytes = world.encode_payload().expect("encode_payload failed");
    let schema = world.schema_descriptor();
    let concept_registry = world.registry().expect("registry failed");
    let compiled = compile(&payload_bytes, &schema, &concept_registry).expect("compile failed");

    // Step 2: Prepare apply inputs.
    // SET_SLOT(layer=0, slot=0, value=Code32(1,0,1)) — the "forum" goal.
    let op_code = OP_SET_SLOT;
    let op_args = set_slot_args(0, 0, sterling_kernel::carrier::code32::Code32::new(1, 0, 1));
    let operator_registry = kernel_operator_registry();

    // Write pre-state bytes.
    std::fs::write(output_dir.join("pre_state_identity.bin"), compiled.state.identity_bytes())
        .expect("failed to write pre_state_identity.bin");
    std::fs::write(output_dir.join("pre_state_status.bin"), compiled.state.status_bytes())
        .expect("failed to write pre_state_status.bin");

    // Write apply inputs.
    std::fs::write(output_dir.join("op_code.bin"), op_code.to_le_bytes())
        .expect("failed to write op_code.bin");
    std::fs::write(output_dir.join("op_args.bin"), &op_args)
        .expect("failed to write op_args.bin");

    // Step 3: Apply.
    let (post_state, step_record) =
        apply(&compiled.state, op_code, &op_args, &operator_registry).expect("apply failed");

    // Write post-state bytes.
    std::fs::write(output_dir.join("post_state_identity.bin"), post_state.identity_bytes())
        .expect("failed to write post_state_identity.bin");
    std::fs::write(output_dir.join("post_state_status.bin"), post_state.status_bytes())
        .expect("failed to write post_state_status.bin");

    // Write step record fields.
    std::fs::write(output_dir.join("step_record_op_code.bin"), step_record.op_code)
        .expect("failed to write step_record_op_code.bin");
    std::fs::write(output_dir.join("step_record_op_args.bin"), &step_record.op_args)
        .expect("failed to write step_record_op_args.bin");
    std::fs::write(
        output_dir.join("step_record_result_identity.bin"),
        &step_record.result_identity,
    )
    .expect("failed to write step_record_result_identity.bin");
    std::fs::write(
        output_dir.join("step_record_result_status.bin"),
        &step_record.result_status,
    )
    .expect("failed to write step_record_result_status.bin");

    // Step 4: Compute expected hashes.
    let post_identity_digest =
        canonical_hash(HashDomain::IdentityPlane, &post_state.identity_bytes());
    let post_evidence_digest =
        canonical_hash(HashDomain::EvidencePlane, &post_state.evidence_bytes());

    let hashes_json = canonical_json_bytes(&serde_json::json!({
        "post_identity_digest": post_identity_digest.as_str(),
        "post_evidence_digest": post_evidence_digest.as_str(),
        "pre_identity_digest": compiled.identity_digest.as_str(),
        "pre_evidence_digest": compiled.evidence_digest.as_str(),
    }))
    .expect("hashes canonical JSON failed");
    std::fs::write(output_dir.join("expected_hashes.json"), &hashes_json)
        .expect("failed to write expected_hashes.json");

    println!("pre_identity_digest={}", compiled.identity_digest.as_str());
    println!("pre_evidence_digest={}", compiled.evidence_digest.as_str());
    println!("post_identity_digest={}", post_identity_digest.as_str());
    println!("post_evidence_digest={}", post_evidence_digest.as_str());
    println!(
        "pre_identity_bytes={}",
        compiled.state.identity_bytes().len()
    );
    println!(
        "post_identity_bytes={}",
        post_state.identity_bytes().len()
    );
    println!(
        "step_record_op_args={}",
        step_record.op_args.len()
    );
    println!("golden written to: {}", output_dir.display());
}
