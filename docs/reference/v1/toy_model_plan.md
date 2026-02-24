# Toy Model Plan: Micro-Domain Linguistic LLM on Apple Silicon

## Target Domain and Scope: Linguistic Explanations

For this toy model, a **micro-domain of linguistic explanations and syntactic/semantic descriptions** is ideal. This focused scope keeps the knowledge graph and rules manageable while still requiring rich reasoning. Key characteristics of this domain include:

* **Linguistic Concepts as Terms:** The model will handle concepts like _noun phrase (NP)_, _verb phrase (VP)_, _determiner_, _predicate nominal_, _head vs. modifier_, _dependency relations_, etc. By constraining terms to linguistic theory, we ensure a small but meaningful vocabulary.
* **Rule-Based Structure:** Linguistics naturally involves formal rules (e.g. word order constraints, agreement). This domain aligns with rule-based reasoning: the model can enforce principles like Hawkins’ _Early Immediate Constituent (EIC)_ efficiency constraint[davidtemperley.com](https://davidtemperley.com/wp-content/uploads/2015/11/temperley-jql08.pdf#:~:text=Hawkins%E2%80%99%20research%20brings%20together%20a,access%E2%80%99%E2%80%99%20to%20the%20children%20of) or specific grammar rules for predicative nominal constructions. The structured nature of language makes it a “sandbox” where rules guide reasoning instead of fighting it.
* **Bounded Reasoning Tasks:** Within this scope, queries can be _definitions_ (“What is a predicate nominal?”), _explanations_ (“Why is ‘X is Y’ a predicative nominal construction?”), or _constraint checks_ (“Does this sentence satisfy Hawkins’ EIC principle?”). These require logical reasoning but in a controlled setting with limited concepts.
* **Fit with Seed Studies:** This domain directly ties into the user’s seed studies (e.g. Hawkins on efficiency, Colen on predicative nominals, cross-linguistic comparisons). Focusing here means the model’s knowledge graph (KG) can be populated from those sources, making the toy model immediately relevant and easier to validate.

_Why not a broader domain like general language or science?_ Because a general domain would explode the KG size and require extensive factual grounding. Linguistics, by contrast, provides a **structured, compact domain** where we can leverage formal rules and avoid open-ended world knowledge. It's rich enough to be challenging, yet constrained enough to remain tractable.

## Core Infrastructure: RGBA Codes as Pure 4-D Symbolic Tokens

### The Fundamental Insight

If you own the whole stack, you absolutely don't *need* "images" in the human sense. You really care about the 4-tuple in [0,255]⁴; PNG/JPEG, gamma, color spaces, all of that is just legacy baggage.

Sterling's architecture treats RGBA not as visual data but as **pure symbolic codes**. The pipeline conceptually becomes:

* You have an ID space: `id ∈ {0,…,2³²-1}` or separate spaces for `concept_id`, `predicate_id`, etc.
* You map each ID to a 4-tuple: `id ↦ (r,g,b,a) ∈ {0,…,255}⁴`
* Instead of writing those into a PNG or video frame, you:
  * Keep them as a flat tensor of shape `(N, 4)`, or
  * Arrange into `(H, W, 4)` tensors purely for convenience so you can use ViT/convolutions

**No human-visible "image" is needed.** The only reason to *ever* serialize to PNG/JPEG would be:
* Interop with tools that expect real image files, or
* If you want to reuse pretrained vision encoders whose input pipeline assumes normal images.

If you're designing your own encoder/decoder, you can:
* Feed the `(N, 4)` integer tensor directly, or
* Embed each 4-tuple with a small MLP and then apply a transformer/ViT-style block.

### Overhead Eliminated

**You avoid:**
* PNG/JPEG encode/decode
* Color management (gamma, color spaces, interpolation)
* Quantization/clamping that isn't under your control

**You still keep:**
* The *discrete* code space (4 bytes per "pixel" symbol)
* The possibility of 2D spatial layouts (if useful for ViT-style models)

In practice, Sterling uses in-memory arrays of shape `(channels=4, height, width)` directly. The raw arrays go straight into the model, minimizing overhead. The "image" is just one way of arranging and transporting those codes – a convenient carrier format to plug into existing VLM plumbing, not a visual artifact.

### Compression Strategy: Semantic + Numeric, Not Optical

Sterling's compression happens at two levels:

1. **Text → IR** collapses many paraphrases and filler into a canonical structure (semantics, not words).
2. **IR IDs → RGBA + ViT** compresses a potentially long sequence of IDs into a smaller sequence of latents.

This is fundamentally different from DeepSeek-OCR's approach:
* **DeepSeek-OCR:** Uses literal images of text (rendered pages), achieves ~10× compression by turning 1,000 text tokens into ~100 vision tokens. Their compression is "**optical instead of textual**" – they're working in the standard multimodal ecosystem.
* **Sterling:** Directly produces semantically meaningful RGBA codes. Compression is "**semantic + numeric**" – first you throw away unimportant linguistic detail (IR), then you hyper-pack what remains. You're not just compressing representation; you're **changing what is considered the payload** (from text to meaning).

The philosophical move is the same: use a cheap, dense representation to let the model see *more* context / structure for the *same* attention cost. Sterling anchors it in IR and knowledge graphs instead of raw glyph images.

### Architectural Pipeline

Instead of: `text → rendered image → CNN/ViT → vision tokens`

Sterling does: `text → IR → IDs → RGBA codes → symbolic encoder → compressed "semantic tokens"`

Where the encoder can be:
* A ViT-like model over a 2D grid of RGBA codes, or
* A 1D transformer over sequences of 4-D vectors.

You still get a sequence of **compressed tokens** (call them `Z_v`) that stand in for a much longer IR/ID sequence. The difference from DeepSeek: you've already **pre-canonicalized** into IR, so there's less junk to compress. Your "visual" encoder doesn't have to work in pixel space; the patterns are symbolic and designed.

**Core thesis:** Stop worshiping text; treat everything as structured, compressible semantics with a lean reasoning core on top.

## Specific Evaluation Tasks for the Model

To prove out the end-to-end architecture, we define three **core tasks** that the toy model will perform. Each task targets a different part of the pipeline, ensuring that all components (vision encoder, reasoning module, knowledge graph, etc.) are exercised:

1. **Task A — IR Reconstruction (Symbolic ↔ RGBA Code Encoding)**
_Input:_ A sentence or linguistic structure is fed through the pipeline: text → IR → IDs → RGBA codes (4-D integer vectors in [0,255]⁴) → ViT encoder → intermediate representation (IR).
_Output:_ The model must reconstruct the original structured IR from the vision encoder's output. In other words, given the vision-transformed input, recover the symbolic representation of the sentence (e.g. its parse tree or feature structure).
_Purpose:_ This tests the **compression and encoding pipeline**. By training on known sentence-IR pairs, we verify that the ViT-based encoder can preserve enough information in the latent representation to recover the original IR accurately. It validates the ID mapping and the fidelity of symbolic→RGBA codes→latent transformations. Note that we never serialize to PNG/JPEG; the RGBA codes are kept as raw tensors of shape `(N, 4)` or `(H, W, 4)`.
2. **Task B — Rule-Guided Explanation/Definition Generation**
_Input:_ A query combining a compressed context + relevant KG slice + an IR (for a specific sentence or construction). For example: _“Explain why the sentence ‘X is Y’ is a predicate nominal construction.”_
_Output:_ A concise textual explanation, potentially with a few structured reasoning steps or references to rules. For the example, the model might explain that _X is Y_ has a linking verb and a noun that renames the subject, hence it’s a predicate nominal, citing the grammatical rule.
_Purpose:_ This evaluates the **reasoning and natural language generation** modules. The model must retrieve the right facts from the KG (e.g. what a predicate nominal is, what the sentence’s structure is) and apply linguistic rules to form an explanation. Because the domain is narrow and well-defined, the risk of hallucination is low – the model can rely on stored definitions and rules. Success here demonstrates that the system can perform controlled, rule-based NLG (natural language generation) using the grammar/critic module to keep the output valid.
3. **Task C — Logical Consistency Verification**
_Input:_ A claim about linguistic structure or a rule application, plus a compressed context (e.g. a sentence or set of facts). For example: _“Claim: This sentence violates the EIC. Context: \[the sentence’s structure\].”_
_Output:_ A verdict: **Accept**, **Reject**, or **Cannot Verify**, possibly with a brief justification. For instance, the model might check a claim like “X is Y implies X is Z” given KG relations X–Y and Y–Z, and accept it (because if _Y is Z_ and _X is Y_, then _X is Z_ logically).
_Purpose:_ This tests the **verifier/critic ensemble** and multi-hop reasoning. The model must use the KG and rule engine to evaluate consistency of the claim with known facts and rules. It’s effectively a truth-checking or entailment task within the toy domain. This ensures the system can do basic logical inference (e.g. transitive relations, rule satisfaction) and output a confidence judgment.

Together, these three tasks cover the full pipeline: **perception (Task A)**, **reasoned generation (Task B)**, and **logical verification (Task C)**. They are narrow enough to implement on a small model, yet broad enough to demonstrate the thesis that a smaller, rule-informed model can achieve results comparable to larger ones on domain-specific problems.

## Corpus, Knowledge Graph, and Registry Scale

To keep the project **feasible on Apple Silicon** (with limited memory and compute) and easy to debug, we will constrain the sizes of the corpus, concept registry, and knowledge graph:

* **Concept/ID Registry (\~150–250 entries):** This registry will contain all the unique identifiers for concepts, relations, and tokens the model knows. Keeping this around a few hundred items ensures the symbolic code embedding (ID ↦ RGBA 4-tuple in [0,255]⁴) is compact. Each ID maps to a discrete 4-dimensional integer vector: `id ↦ (r,g,b,a) ∈ {0,…,255}⁴`. These RGBA codes are **not images** in the human sense; they are pure symbolic codes that can be arranged as tensors of shape `(N, 4)` or `(H, W, 4)` for ViT processing, but never serialized as PNG/JPEG.
   * _Linguistic concepts:_ \~80–120 entries for grammatical terms (NP, VP, subject, object, complementizer, etc.).
   * _Semantic predicate types:_ \~40–60 entries for abstract relations or frames (e.g. _MOTION_, _IDENTIFICATION_, _STATE_, _HAS-POSSESSION_, which might correspond to verb frames or semantic roles).
   * _Relation types:_ \~20–40 entries for relationships in the KG (such as _is-a_, _part-of_, _has-role_, _satisfies-rule_, _violates-rule_).
   * _Control tokens:_ \~10–20 special identifiers for things like logical operators or padding.
This limited registry size keeps the **ID mapping grid** small and the ViT’s input space bounded. Yet it’s enough to express a wide range of linguistic facts.
* **Toy Corpus (500–1,000 sentences):** We will curate a small corpus of sentences and short texts that exemplify the targeted linguistic phenomena. Quality matters more than quantity here. The corpus will include:
   * **Constructed examples** illustrating key concepts (e.g. various predicative nominal sentences, subject/object alternations, sentences demonstrating Hawkins’ efficiency constraints).
   * **Sentences from the seed studies**: Simplified excerpts or examples referenced in Hawkins’ and Colen’s papers to ground the model in real analyses.
   * **Natural variations**: A few everyday sentences (“The cat ran fast.”, “Kitty zoomed.”) to ensure the model isn’t overfit to only academic examples.
   This size is sufficient to fine-tune components like the IR extractor, the ViT autoencoder (for text-to-image encoding), the realizer (text generator), and to populate the knowledge graph. Yet it’s small enough to train on a CPU or modest GPU, and doesn’t demand large-scale pretraining.
* **Knowledge Graph (300–500 nodes, \~1,000–2,000 edges):** The KG will be a _mini knowledge base_ capturing the essential relationships in our domain. Estimated contents:
   * Nodes for each linguistic concept and each example sentence or entity in the corpus. For instance, a node for _Predicate Nominal_ and nodes for each sample sentence (linked to their structures).
   * Edges encoding relations like _“NP is-a Constituent”_, _“Subject part-of Clause”_, or _“Sentence123 satisfies EIC”_. We’ll also include semantic relations (e.g. \*“cat” is-a _noun_, _has-role agent_ in a sentence).
   * **External knowledge subset:** A small curated slice of WordNet and Wikidata relevant to linguistics. For example, WordNet (a lexical database of English words and their relationships)[sciencedirect.com](https://www.sciencedirect.com/science/article/abs/pii/B9780444595195500885#:~:text=,was%20identified%20as%20a) can supply basic lexical relations and synonyms for terms like _noun, verb, animal_. Likewise, a focused subset of Wikidata could provide properties of linguistic categories or known linguistic examples. By cherry-picking only what’s needed (perhaps a few dozen entries), we avoid bloat.
A few hundred nodes and a couple thousand relations is **small enough to reason over** on a single machine, but rich enough to test multi-hop reasoning. The KG can be stored in memory as simple Python objects or a lightweight graph database, without taxing the system. It will also be used to create prompts and check consistency in Task C.

Overall, these scales (hundreds of concepts/nodes, \~1k sentences) ensure that the entire system remains **debuggable and fast** on local hardware. It avoids the need for distributed systems or cloud resources, staying true to the goal of an at-home, Apple Silicon-compatible research prototype.

## Apple Silicon Constraints and Optimizations

A core design goal is that this toy model runs efficiently on Apple hardware (M1/M2/M3 chips) by leveraging **MPS (Metal Performance Shaders)** and/or **MLX** for Mac/desktop deployment, with **CoreML** as a secondary target for mobile/iOS. This imposes specific constraints on model size and design, which we embrace:

* **Model Size and Architecture Choices:**
   * _Vision Encoder:_ Use a small Vision Transformer (ViT). A ViT-Tiny or ViT-S (patch size 16) with on the order of 30 million parameters or less is ideal. Models of this size easily fit on the ANE and GPU, and Core ML supports them well. Despite being small, ViTs in this range can achieve strong results on structured image inputs.
   * _Decoder (Language Model):_ Aim for a micro-LLM in the 30M–150M parameter range. This could be a distilled GPT-2 variant or a custom transformer decoder. This size is **magnitudes smaller** than typical LLMs, which makes conversion and on-device inference feasible. With quantization (e.g. 16-bit or 8-bit weights) and Core ML optimization, even 100M+ parameter models can run at useful speeds on an M1/M2[machinelearning.apple.com](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=This%20technical%20post%20details%20how,based%20LLMs%20of%20different%20sizes)[machinelearning.apple.com](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=It%20is%20easiest%20to%20begin,sections%20to%20improve%20the%20performance). Keeping it under \~150M ensures we stay within memory limits and can utilize the ANE/GPU for acceleration.
   * For Mac/desktop: Models run natively via PyTorch MPS backend (no conversion needed). MLX can be explored for further optimization.
   * For mobile/iOS: The ViT and decoder will be converted via _coremltools_ into Core ML `.mlmodel` format.
* **Deployment Constraints:** 
   * **MPS/MLX (Primary)**: Standard PyTorch ops work natively on Apple Silicon via MPS.
   * **CoreML (Mobile)**: We will design models to **avoid ops or patterns that Core ML doesn't support well**. For example:
   * Use standard layers like Linear (fully connected), Conv2D, GELU activation, LayerNorm, etc., which Core ML handles efficiently on ANE. Avoid exotic or unsupported ops (e.g. certain custom layernorms or dynamic position embeddings).
   * Prefer **fixed input shapes** for all models. For instance, the RGBA code grid that encodes the IR will have a fixed size (small height × width), and the decoder will use a fixed context length for generation. Core ML _can_ support flexible shapes in some cases, but fixed shapes simplify conversion and allow more of the model to stay on ANE/GPU[machinelearning.apple.com](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=It%20is%20easiest%20to%20begin,sections%20to%20improve%20the%20performance)[machinelearning.apple.com](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=To%20make%20the%20model%20exportable,LlamaForCausalLM%20module%2C%20as%20shown%20below). We'll pad or batch inputs to fit the expected shapes rather than rely on truly dynamic dimensions. Note that the RGBA codes are kept as raw `uint8` tensors, never serialized to image formats.
   * No dynamic control flow in the exported model. Any looping or conditional logic (for reasoning steps) will be handled in Python/CPU, not inside the neural network graph. This means things like the planning/controller module will run outside the Core ML models.
* **Memory and Compute Budget:** Apple Silicon devices (like a MacBook with 8–16 GB unified memory) have limited RAM compared to servers. We target a total memory footprint of **< 3 GB** for the running system, leaving headroom for the OS and other processes. To achieve this:
   * Use **low-dimensional embeddings** for knowledge graph entities (likely 64–128 dimensions) so that storing all concept embeddings in memory is cheap. We don’t need large 768-dim embeddings for a few hundred concepts.
   * Keep the rule engine lightweight – a simple Python-based evaluator or a mini logic engine, rather than a heavy Prolog or large database server. The logic rules (like pattern matching for grammar) can be coded in a straightforward way that uses minimal overhead.
   * Optimize the batch sizes and caching. For example, run the ViT and decoder with batch size 1 (since most queries will be one at a time), and utilize the ANE’s strength in matrix ops for those single inputs. Caching of intermediate results (like storing the IR of the corpus examples) can avoid recomputation without needing big caches.
* **ANE and GPU Utilization:** We strategically assign model parts to the hardware component that suits them:
   * The **ViT encoder** (processing RGBA code tensors) will run on the ANE if possible. ANE is excellent for convolution and transformer operations on moderate size tensors, giving a big speed-up for the encoding pipeline. Since we're working with raw `uint8[4]` vectors rather than decoded images, there's no image I/O overhead.
   * The **decoder LLM** can run on the GPU or ANE depending on size. Very small models might run fully on ANE. For slightly larger (100M+), the GPU may handle the bulk while ANE could assist with parts of the graph. Apple’s Core ML runtime will automatically distribute the workload across CPU/GPU/ANE for optimal throughput[machinelearning.apple.com](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=This%20technical%20post%20details%20how,based%20LLMs%20of%20different%20sizes). The key is we’ve ensured the model uses compatible layers so it can take advantage of these neural engines.
   * The **knowledge graph queries and rule evaluations** will run on the CPU (as they involve non-tensor operations and irregular logic). This is fine, since those operations are not heavy matrix math – they’re more about traversing a small graph or applying a few logical rules, which even a single CPU core can handle quickly for our scale.
* **Tooling and Deployment:** 
   * **Mac/Desktop**: Use PyTorch with MPS backend (native, no conversion). Explore MLX for further optimization.
   * **Mobile/iOS**: Use _coremltools_ for conversion to `.mlpackage` format. Possibly Create ML or on-device testing.
   * For any custom operations (if absolutely needed), we have the option of using Metal Performance Shaders or writing a tiny Metal kernel, but the goal is to avoid that by sticking to standard ops. 
   * The end result should be a self-contained app or script that runs **entirely on Apple Silicon hardware**, with no external requirements like CUDA or cloud services.

In summary, every design choice (from model size to input format) is made with the Apple Silicon **hardware reality** in mind. By keeping models small and ops simple, we ensure the entire pipeline – ViT encoding, KG lookups, rule-based reasoning, and LLM decoding – executes smoothly on a Mac’s CPU/GPU/ANE combo. This way, we prove the thesis that a carefully distilled model can be **efficiently run at home** while still delivering robust, rule-informed linguistic reasoning comparable to much larger models in broader domains.

Citations

[![](https://www.google.com/s2/favicons?domain=https://davidtemperley.com&sz=32)NJQL\_A\_316117 256..282 ++https://davidtemperley.com/wp-content/uploads/2015/11/temperley-jql08.pdf](https://davidtemperley.com/wp-content/uploads/2015/11/temperley-jql08.pdf#:~:text=Hawkins%E2%80%99%20research%20brings%20together%20a,access%E2%80%99%E2%80%99%20to%20the%20children%20of)[![](https://www.google.com/s2/favicons?domain=https://www.sciencedirect.com&sz=32)Application of semantic and lexical analysis to technology ...https://www.sciencedirect.com/science/article/abs/pii/B9780444595195500885](https://www.sciencedirect.com/science/article/abs/pii/B9780444595195500885#:~:text=,was%20identified%20as%20a)[![](https://www.google.com/s2/favicons?domain=https://machinelearning.apple.com&sz=32)On Device Llama 3.1 with Core ML - Apple Machine Learning Researchhttps://machinelearning.apple.com/research/core-ml-on-device-llama](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=This%20technical%20post%20details%20how,based%20LLMs%20of%20different%20sizes)[![](https://www.google.com/s2/favicons?domain=https://machinelearning.apple.com&sz=32)On Device Llama 3.1 with Core ML - Apple Machine Learning Researchhttps://machinelearning.apple.com/research/core-ml-on-device-llama](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=It%20is%20easiest%20to%20begin,sections%20to%20improve%20the%20performance)[![](https://www.google.com/s2/favicons?domain=https://machinelearning.apple.com&sz=32)On Device Llama 3.1 with Core ML - Apple Machine Learning Researchhttps://machinelearning.apple.com/research/core-ml-on-device-llama](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=To%20make%20the%20model%20exportable,LlamaForCausalLM%20module%2C%20as%20shown%20below)

All Sources

[![](https://www.google.com/s2/favicons?domain=https://davidtemperley.com&sz=32)davidtemperley](https://davidtemperley.com/wp-content/uploads/2015/11/temperley-jql08.pdf#:~:text=Hawkins%E2%80%99%20research%20brings%20together%20a,access%E2%80%99%E2%80%99%20to%20the%20children%20of)[![](https://www.google.com/s2/favicons?domain=https://www.sciencedirect.com&sz=32)sciencedirect](https://www.sciencedirect.com/science/article/abs/pii/B9780444595195500885#:~:text=,was%20identified%20as%20a)[![](https://www.google.com/s2/favicons?domain=https://machinelearning.apple.com&sz=32)machinel...ing.apple](https://machinelearning.apple.com/research/core-ml-on-device-llama#:~:text=This%20technical%20post%20details%20how,based%20LLMs%20of%20different%20sizes)
