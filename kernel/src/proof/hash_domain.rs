//! Typed domain separators for canonical hashing.
//!
//! Every production hash computation MUST select a domain via [`HashDomain`].
//! This module is the single authority for domain-separator byte strings.
//! Adding a new domain is a single change here — the enum, `as_bytes()`,
//! `ALL`, and `Display` are all generated from the same macro invocation.

/// Declares `HashDomain` enum, `as_bytes()`, `ALL`, and `Display` from one list.
macro_rules! define_hash_domains {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $bytes:expr
        ),+ $(,)?
    ) => {
        /// Typed domain separator for [`super::hash::canonical_hash`].
        ///
        /// Every variant maps to a unique, null-terminated byte string used as
        /// a SHA-256 prefix. The byte values match v1 exactly where applicable.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum HashDomain {
            $(
                $(#[$meta])*
                $variant,
            )+
        }

        impl HashDomain {
            /// The raw domain-separator bytes (null-terminated).
            #[must_use]
            pub const fn as_bytes(&self) -> &'static [u8] {
                match self {
                    $( Self::$variant => $bytes, )+
                }
            }

            /// All domain variants in declaration order.
            ///
            /// Generated from the same macro invocation as the enum — cannot diverge.
            pub const ALL: &[HashDomain] = &[
                $( Self::$variant, )+
            ];
        }

        impl core::fmt::Display for HashDomain {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $( Self::$variant => write!(f, stringify!($variant)), )+
                }
            }
        }
    };
}

define_hash_domains! {
    // -----------------------------------------------------------------------
    // Kernel (carrier layer)
    // -----------------------------------------------------------------------

    /// `ByteStateV1` identity plane hashing (deduplication, cycle detection).
    IdentityPlane => b"STERLING::BYTESTATE_IDENTITY::V1\0",

    /// `ByteStateV1` evidence hashing (identity + status for replay verification).
    EvidencePlane => b"STERLING::BYTESTATE_EVIDENCE::V1\0",

    /// `ByteTraceV1` payload hashing.
    ByteTrace => b"STERLING::BYTETRACE::V1\0",

    /// Registry snapshot hashing.
    RegistrySnapshot => b"STERLING::REGISTRY_SNAPSHOT::V1\0",

    /// Schema bundle hashing.
    SchemaBundleHash => b"STERLING::BYTESTATE_SCHEMA_BUNDLE::V1\0",

    /// Compilation payload commitment (Native-originated).
    CompilationPayload => b"STERLING::COMPILATION_PAYLOAD::V1\0",

    /// Step hash chain: initial step commitment.
    TraceStep => b"STERLING::TRACE_STEP::V1\0",

    /// Step hash chain: chained step commitment.
    TraceStepChain => b"STERLING::TRACE_STEP_CHAIN::V1\0",

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    /// Search node fingerprint hashing.
    SearchNode => b"STERLING::SEARCH_NODE::V1\0",

    /// Search candidate action hashing.
    SearchCandidate => b"STERLING::SEARCH_CANDIDATE::V1\0",

    /// Search graph hashing.
    SearchGraph => b"STERLING::SEARCH_GRAPH::V1\0",

    /// Search tape header hashing (chain seed).
    SearchTape => b"STERLING::SEARCH_TAPE::V1\0",

    /// Search tape chain step hashing.
    SearchTapeChain => b"STERLING::SEARCH_TAPE_CHAIN::V1\0",

    // -----------------------------------------------------------------------
    // Harness
    // -----------------------------------------------------------------------

    /// Bundle artifact content hashing.
    BundleArtifact => b"STERLING::BUNDLE_ARTIFACT::V1\0",

    /// Bundle digest (normative projection).
    BundleDigest => b"STERLING::BUNDLE_DIGEST::V1\0",

    /// Harness fixture schema hashing.
    HarnessFixture => b"STERLING::HARNESS_FIXTURE::V1\0",

    /// Codebook hash (concept registry canonical form).
    CodebookHash => b"STERLING::CODEBOOK_HASH::V1\0",

    /// Policy snapshot hashing.
    PolicySnapshot => b"STERLING::POLICY_SNAPSHOT::V1\0",

    /// Suite identity hashing (`world_id` binding).
    SuiteIdentity => b"STERLING::SUITE_IDENTITY::V1\0",

    // -----------------------------------------------------------------------
    // Benchmarks
    // -----------------------------------------------------------------------

    /// Benchmark input hashing.
    BenchInput => b"STERLING::BENCH_INPUT::V1\0",

    /// Benchmark determinism guard.
    BenchGuard => b"STERLING::BENCH_GUARD::V1\0",
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn all_variants_in_all_constant() {
        // ALL is generated from the same macro — this is a structural guarantee.
        // We verify the count here as a human-readable anchor.
        assert_eq!(
            HashDomain::ALL.len(),
            21,
            "expected 21 domain variants in ALL"
        );
    }

    #[test]
    fn all_bytes_unique() {
        let mut seen = BTreeSet::new();
        for domain in HashDomain::ALL {
            assert!(
                seen.insert(domain.as_bytes()),
                "duplicate domain bytes: {domain}"
            );
        }
    }

    #[test]
    fn all_null_terminated() {
        for domain in HashDomain::ALL {
            assert!(
                domain.as_bytes().ends_with(&[0]),
                "{domain} is not null-terminated"
            );
        }
    }

    #[test]
    fn all_follow_naming_convention() {
        for domain in HashDomain::ALL {
            let bytes = domain.as_bytes();
            assert!(
                bytes.starts_with(b"STERLING::"),
                "{domain} does not start with STERLING::"
            );
            assert!(
                bytes.ends_with(b"::V1\0"),
                "{domain} does not end with ::V1\\0"
            );
        }
    }

    #[test]
    fn display_returns_variant_name() {
        assert_eq!(format!("{}", HashDomain::IdentityPlane), "IdentityPlane");
        assert_eq!(format!("{}", HashDomain::BundleArtifact), "BundleArtifact");
    }
}
