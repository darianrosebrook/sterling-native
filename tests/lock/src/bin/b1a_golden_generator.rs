//! Generator for B1a golden bundle fixture.
//!
//! Runs `RomeMiniSearch` with `SearchPolicyV1::default()` and `Uniform` scorer,
//! then writes the bundle directory to the specified output path.
//!
//! Usage: `b1a_golden_generator <output_dir>`
//!
//! The output directory must not exist or must be empty.

use sterling_harness::bundle_dir::write_bundle_dir;
use sterling_harness::runner::{run_search, ScorerInputV1};
use sterling_harness::worlds::rome_mini_search::RomeMiniSearch;
use sterling_search::policy::SearchPolicyV1;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: b1a_golden_generator <output_dir>");
        std::process::exit(1);
    }
    let output_dir = std::path::Path::new(&args[1]);

    // Create output directory if it doesn't exist.
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir).expect("failed to create output directory");
    }

    let policy = SearchPolicyV1::default();
    let bundle =
        run_search(&RomeMiniSearch, &policy, &ScorerInputV1::Uniform).expect("search run failed");

    write_bundle_dir(&bundle, output_dir).expect("failed to write bundle directory");

    println!("bundle_digest={}", bundle.digest.as_str());
    println!("artifact_count={}", bundle.artifacts.len());
    for (name, art) in &bundle.artifacts {
        println!("artifact={} hash={} normative={}", name, art.content_hash.as_str(), art.normative);
    }
    println!("golden written to: {}", output_dir.display());
}
