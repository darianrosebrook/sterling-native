# M2 ByteTrace Binary Format Specification

> Reference: v1 `core/carrier/bytetrace.py`. This spec ports the wire format
> byte-for-byte and adds the commitment scheme needed for divergence localization.

## Binary layout (`.bst1` format)

```
[envelope_len:u16le][envelope:JSON]       ← NOT hashed (observability only)
[magic:4 = "BST1"]                        ← hashed
[header_len:u16le][header:canonical JSON]  ← hashed
[body: fixed-stride frames]               ← hashed
[footer_len:u16le][footer:canonical JSON]  ← hashed
```

### Envelope (not hashed)

Non-deterministic observability metadata. Excluded from all hashes.

```json
{
  "timestamp": "<ISO 8601>",
  "trace_id": "<uuid>",
  "runner_version": "<string>",
  "wall_time_ms": <integer>
}
```

### Header (hashed, canonical JSON)

Commits the schema and registry for the entire trace. Canonical JSON
(ASCII keys, sorted, compact, integers only — same rules as `canon.rs`).

```json
{
  "arg_slot_count": <integer>,
  "codebook_hash": "<content_hash>",
  "domain_id": "<string>",
  "fixture_hash": "<content_hash>",
  "layer_count": <integer>,
  "registry_epoch_hash": "<content_hash>",
  "schema_version": "<string>",
  "slot_count": <integer>,
  "step_count": <integer>
}
```

The header is a **descriptor only**. It does not encode state content.
JSON is allowed here because it describes metadata, not trace frames
(S1-M2-NO-SECOND-TRUTH).

### Body (hashed, binary)

Fixed-stride frames concatenated in execution order. Frame 0 is the
initial state (op_code = `INITIAL_STATE` sentinel, op_args = zero-filled).

**Frame layout** (constant stride across all frames in a trace):

```
[op_code:4]                          4 bytes   Code32 as LE bytes
[op_args:arg_slot_count * 4]         variable  padded Code32 args
[identity_plane:layers*slots*4]      variable  identity bytes
[status_plane:layers*slots]          variable  status bytes
```

**Frame stride** = 4 + (arg_slot_count * 4) + (layer_count * slot_count * 4) + (layer_count * slot_count)

This is fixed and derivable from the header. The body length must equal
`step_count * frame_stride`. Reader rejects any other length.

No JSON inside frames. No serde. Pure byte transforms.

### Footer (hashed, canonical JSON)

```json
{
  "suite_identity": "<content_hash>"
}
```

Optional field `witness_store_digest` may be present when a witness
sidecar exists. When absent, the field is omitted entirely (not null).

## Payload hash (whole-trace digest)

The whole-trace payload hash is the primary claim surface:

```
sha256(DOMAIN_BYTETRACE || magic || header_json || body || footer_json)
```

This matches v1's `compute_payload_hash()` exactly. The envelope is
excluded. The magic bytes are included.

## Step hash chain (divergence localization)

The step hash chain enables O(1) divergence localization without
rereading the entire trace:

```
chain_0 = sha256(DOMAIN_TRACE_STEP || frame_0_bytes)
chain_i = sha256(DOMAIN_TRACE_STEP_CHAIN || chain_{i-1} || frame_i_bytes)
```

The final chain value `chain_{n-1}` is the **step chain digest**. It is
recorded alongside the payload hash but is a separate claim surface.

This requires two new Native-originated domain prefixes:
- `DOMAIN_TRACE_STEP = b"STERLING::TRACE_STEP::V1\0"`
- `DOMAIN_TRACE_STEP_CHAIN = b"STERLING::TRACE_STEP_CHAIN::V1\0"`

These are not V1-oracle-originated (v1 has no step chain). They follow
the same convention (null-terminated, SHA-256).

## Replay verification

`replay_verify(trace_bundle)` recomputes the trace from the initial
state by re-applying each operator and comparing frame-by-frame:

1. Parse the binary trace (reject malformed: bad magic, truncation,
   wrong body length, invalid status bytes in frames).
2. Extract initial state from frame 0.
3. For each subsequent frame i:
   a. Apply the recorded operator to the current state.
   b. Compare resulting identity + status bytes to frame i.
   c. If they differ: return `Divergence { frame_index: i, detail }`.
   d. If they match: advance state, continue.
4. After all frames: recompute payload hash and compare to committed
   digest. If it matches: return `Match`.

The step chain provides localization hints but is not required for
correctness — frame-by-frame comparison is the primary verification.

## Strictness rules

- Reader rejects truncated input (body_len != step_count * stride)
- Reader rejects unknown magic bytes
- Reader rejects frames with invalid SlotStatus discriminants
- Reader rejects header with non-positive dimensions
- Writer rejects frames that don't match header dimensions
- No partial traces: write is all-or-nothing (build, then serialize)

## What M2 does NOT include

- Multi-world orchestration
- Trace compression
- Streaming writer (traces are small enough to buffer)
- General operator catalog (M2 uses one toy operator)
