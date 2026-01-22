Prompt for your stronger LLM to perform the higher-risk design refactors

Goal
- Safely apply higher-risk Clippy/design suggestions across the repo to reduce warnings and improve maintainability while preserving behavior and tests. Produce small, reviewable patches and run Clippy + tests after each change.

Context (repo)
- Rust project at repo root; primary binary `sassy-browser`. Tests currently pass.
- Notable remaining Clippy/design issues: large_enum_variant, too_many_arguments, type_complexity, result_large_err, upper_case_acronyms, redundant_pattern_matching, collapsible_match, and a few field_reassign_with_default cases already partially handled.
- Key example files: `src/browser/tab.rs` (large enum variants like `File(OpenFile)`), `src/ui/render.rs` and `src/paint.rs` (many-argument drawing APIs), `src/http_client.rs` (large Err variant), `src/rest_client.rs` and `src/mcp.rs` (upper-case acronyms), various modules with complex types like `src/js/dom.rs`.

Requirements / Constraints
1. Safety first: All changes must preserve behavior. After each logical change, run:
   - `cargo clippy --all-targets --all-features`
   - `cargo test --workspace`
   If tests fail or behavior is ambiguous, stop and explain the failure and safer alternatives.
2. Minimize public API breakage. If a change is potentially breaking (public types/enum variant names, serde formats), prefer non-breaking alternatives (e.g., boxing large fields inside variants while keeping variant names; use `#[serde(rename = "...")]` where necessary).
3. Small, reviewable commits/patches: produce patch diffs in the apply_patch-style unified format (or a short PR with per-file patches). Limit each patch to a small focused change (e.g., one enum boxing, or refactor of a single function).
4. Provide per-change rationale: include size/memory tradeoffs, compatibility impact, and a test plan for each change.
5. Keep compile/test iteration fast: prefer mechanical edits that are straightforward first (boxing large fields), then refactors that require more design (function signature grouping).
6. If a proposed change is risky or requires domain knowledge (e.g., semantics of Tab/OpenFile lifetime ownership), present 2 options with pros/cons and an explicit recommended choice.

Concrete tasks (in order)
1. Large enum variants
   - Identify large enums (Clippy flagged `src/browser/tab.rs`), and inline-suggest boxing large fields.
   - Example: change `File(OpenFile)` → `File(Box<OpenFile>)`. Update any construction sites and pattern matches. Ensure `Box` usage preserves borrow semantics.
   - After each enum change: run clippy/tests and produce a short summary (size reduction, compile diffs).
2. Result large error
   - For functions returning `Result<T, ureq::Error>` (e.g., `src/http_client.rs`), propose converting to `Result<T, Box<ureq::Error>>` or wrap/mapping errors to a compact local error type. Explain memory/runtime tradeoffs and backward-compatibility costs.
   - Implement the safer, minimal-change option first (box the error) and validate tests.
3. Too many arguments
   - For functions exceeding the argument threshold (e.g., `src/paint.rs`, `src/ui/render.rs`), propose grouping arguments into small structs (e.g., `Rect`, `DrawOptions`) or use builder structs.
   - Provide a patch for one representative function (smallest impact, with internal/private visibility) showing the refactor and all updated call-sites.
4. Type complexity
   - For very complex types (e.g., `Rc<RefCell<HashMap<(u64, String), Vec<Value>>>>` in `src/js/dom.rs`), propose type aliases or small newtype wrappers with intuitive names. Implement aliases where local and non-breaking.
5. Upper-case acronyms
   - For enums like `GET`, `POST` in `src/rest_client.rs` and `XAI` in `src/mcp.rs`, recommend rename to `Get`, `Post`, `Xai`. If these are serialized/deserialized externally, add `#[serde(rename = "GET")]` to preserve external JSON/text APIs.
   - Provide a patch for one enum showing rename + serde preservation.
6. Redundant pattern matching & collapsible matches
   - Replace `if let Ok(_) = expr` with `if expr.is_ok()` where drop-order is not semantically important.
   - Collapse nested `if let`/match patterns where safe.
   - Implement a small, representative batch with tests.
7. Field reassignment with default
   - Continue replacing `let mut x = Type::default(); x.field = value;` with `let x = Type { field: value, ..Default::default() }` where local and safe.
8. Deliverables for each change
   - Patch in apply_patch-style diff for the modified files.
   - Short rationale (2–4 lines): why change, impact, backward-compatibility note.
   - Commands you ran and results:
     - `cargo clippy --all-targets --all-features`
     - `cargo test --workspace`
   - If tests fail, include failing test names and proposed fixes or rollbacks.

Edge cases & questions
- If changing enum variant names that are part of a public API or persisted format, do not rename without explicit mapping; instead box fields or add serde renames.
- If boxing large fields causes significant performance regressions in hot paths, measure and report (size and simple microbenchmark suggestion). Provide an alternative (e.g., use `Arc` or `Rc` instead of `Box`) with justification.
- If a refactor requires changing many call-sites across unrelated modules, split into an initial compatibility shim + phased migration.

Output format (what I want from you)
- For each patch: a short header, the apply_patch diff, the rationale, and the exact commands you ran with outputs (clippy/tests). Keep each patch small.
- If a change is risky, include an explicit "STOP" and a precise list of the call-sites or public APIs that need owner review.

Finish by summarizing overall effect (warnings reduced, tests passing, size/memory notes) and recommended next refactors.

If anything above is ambiguous, list the minimal clarifying questions before making any code edits.

— end prompt —

Use that prompt to run the intensive LLM. Want me to: 
- run the LLM here (I can try another automated pass), or 
- save this prompt to a file (`LLM_PROMPT_HIGH_RISK.md`) in the repo?