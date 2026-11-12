# Playbook: a Crucible campaign on a Solana program

One generic procedure from earlier Crucible campaigns on a small Anchor vault and on large
programs. Lesson IDs `Kn` point to `lessons.md`. Pitfall IDs (B, R, C, P, T, S, V) point to
`crucible-pitfalls.md`. Invariant classes point to `invariant-catalog.md`. Code patterns are in
`templates/README.md`.

The procedure has 20 steps in 6 stages:

| Stage | Steps | Output |
|-------|-------|--------|
| A. Frame | 1-2 | Protocol and phase plan |
| B. Understand | 3-4 | Intake note, tier table |
| C. Design | 5-7 | Reviewed spec, reviewed plan |
| D. Build | 8-12 | Harness that passes the dry run |
| E. Run | 13-17 | Blocker loop, main run, probe pass |
| F. Evaluate | 18-20 | Triage, coverage, notes, scoring |

Working mode: the user decides who runs which command (`anchor`, `avm`, `cargo check`,
`cargo test --lib`, `cargo build-sbf` and each `crucible` command). Claude writes the code. Mark
the steps that the user runs in the plan (step 7).

---

## Stage A. Frame

### Step 1. Choose the protocol: open or blind

Use the blind protocol when an audit report exists and you want to measure what the fuzzer finds
without it.

Rules for the blind phase:

1. Do not read the audit report. Do not search for it or for public write-ups of the target.
2. Do not read diffs of audit-fix commits (`git show`, `git diff`, `git log -p`). Read only the
   current code and docs.
3. Do not open files that the user marks as report material. If unsure, ask first.
4. Take phase A invariants only from the docs and from generic classes. Put your own code
   suspicions in a sealed note (step 15).
5. Record each design decision with its reason. The final comparison then shows why a bug was or
   was not reachable.
6. Give review subagents the same blind rules (K47).

Practical help: a move of the target's `.git` folder out of the working tree removes the
audit-fix commits from view.

### Step 2. Plan the phases (optional, blind protocol)

| Phase | Who | Content |
|-------|-----|---------|
| A. Blind build and first run | Claude + user | Steps 3-19. Record all crashes and their triage. |
| B. Offline review | User only | The user compares phase A with the report. Claude gets no report details. |
| C. Further experiments | Claude + user | Changes that the user asks for, and the sealed-note invariants. Record for each change if it came from a report hint. |
| D. Final comparison | Claude + user | The user shares the report. Fill the scoring table (step 20). |

---

## Stage B. Understand

### Step 3. Target intake

Write an intake note (`01-target-intake.md`) with these items:

1. **Commit.** The audited commit, not the fixed one. The user confirms it.
2. **Identity.** Program name = `[lib] name` (K1). Program ID. Size (lines, instructions).
3. **Framework and versions.** Anchor version (`Cargo.toml`, `Cargo.lock`), IDL spec version
   (0.30 and 0.31 differ, K3, B3), Rust toolchain of the program, versions of external crates.
   Native programs without an IDL: `ctx.raw_call(Instruction { .. })`. Anchor below 0.30: plan
   the IDL path here (K49, B5).
4. **Build.** `avm use <version>`, then check `anchor --version` (K2, B1). `anchor build`. If only
   the IDL step fails, use the committed IDL after a check against `lib.rs` (B2). Output:
   `target/deploy/<lib>.so`, `target/idl/<lib>.json`. Old Solana toolchains on Apple Silicon:
   K52, B7. Relative paths in the build configuration: K50.
5. **External dependencies.** One row per program: ID, which instructions use it, how (CPI or data
   read only). Also sysvars, clock and epoch use, oracles, Token-2022, hard-coded admin keys,
   initialize instructions without access control (K19). Fixture `.so` files of external
   programs: strip `.symtab` (K51, B6).
6. **External account layouts.** For each external account that the program reads, check the
   struct in the exact crate version from `Cargo.lock`. Count the fields.
7. **Harness skeleton.** From the program root: `crucible init <lib>`; copy the IDL to
   `fuzz/<lib>/idls/`; `crucible run <lib> invariant_test --dry-run` on the empty template.

### Step 4. Partition by external dependency (large targets)

Do this when one harness for all instructions would put every blocker into one long loop (K46).

1. List each instruction with its programs, sysvars, owner constraints and remaining accounts.
2. Group by the most expensive dependency:
   - tier 1: SPL Token only, and accounts that the harness can write (synthetic external data);
   - tier 2: native programs built into LiteSVM, clock and epoch changes;
   - tier 3: CPI into a non-native program with complex state;
   - excluded: programs unrelated to the protocol economics.
3. Draw the state dependency chain between instructions. A tier 2 instruction may process state
   that only tier 2 or 3 creates. A tier can need a complex external program in setup only (K80).
4. Make each tier a sub-project with its own spec, plan and runs.
5. After each tier, record what it reached and which code stays for the next tiers
   (`02-partitioning-large-targets.md`).

Risk to record: a tier boundary can hide cross-tier bugs.

---

## Stage C. Design

### Step 5. Write the design spec

The spec is the single source for the plan and the code. Sections:

| Section | Content |
|---------|---------|
| Goal and limits | Target facts, tier, blind rules |
| Harness structure | Files and their roles (library + binary, `templates/README.md` pattern 2) |
| Fixture and setup | Actors, clock, program objects, shadows, registry (step 9) |
| Environment model | Built-in program versions, clock and epoch (step 10) |
| Actions | One per state change, parameter ranges, flash variants, environment actions (step 11) |
| Invariants | Table with ID, property, source, class; definitions; tolerances (step 12) |
| Expected buckets | Design cases that must fire (step 14) |
| Run sequence | Dry run, blocker loop, sealed note, main run, probe pass, triage, coverage |
| Known risks | Risk, effect, check |

Rules:

- Cite file and line for each program fact and each fuzzer fact.
- State every doc contradiction that you find. Give its case its own invariant ID (K34).
- State the parameter range of each unvalidated config field, with the reason.
- Freeze parameters that would make the harness model ambiguous, and assert them in setup.

### Step 6. Independent model review of the spec

1. Give a separate model the spec, the program source, the fuzzer source and the blind rules.
2. Ask for blockers, major and minor issues, each with a source reference.
3. Check each blocker in the source yourself before you accept it.
4. Disagree with a proposal when it would hide a property. Let the user decide.
5. Write spec version 2 with a change table (finding, change, sections).

See K47.

### Step 7. Implementation plan and its review

1. Write a plan with numbered tasks. Each task has files, interfaces (consumes, produces), steps,
   the exact code, and the compile or test command with its expected output. For patches, see
   K83.
2. Mark which steps the user runs (`[USER]`).
3. Add a "Review focus" list: edge cases with the test or task that pins each one (overlapping
   accounts, zero supply, epoch boundary, large values, layout mismatch).
4. List the deviations from the spec for the user to confirm.
5. Have an independent model review the plan. Apply the accepted findings.

Related lessons: K54 (fixture program has each instruction of the spec), K82 (task order in a
later tier).

---

## Stage D. Build

### Step 8. Subagent-driven execution with controller diff checks

Optional. It keeps the main session context small.

1. The controller (main session) gives one task or a small batch to an implementer subagent.
2. A reviewer subagent checks each batch.
3. The controller reads the diff of every semantic fix against the root cause, not only the
   report (K48).
4. The controller keeps the user steps and the notes.
5. Before the first long run, run a final code review.

### Step 9. Setup design

1. **Order.** Create accounts in the order of the creation graph (K8). PDAs from IDL seeds (K9).
2. **Keys.** Deterministic keypairs, one tag per role, with a tag table (K10).
3. **Relayer.** One funded relayer pays every transaction. Every authority goes into `.signers()`
   (K38). Do not register the relayer as a slot candidate.
4. **Actors.** Admin or authorities plus at least 2 users. Every actor can call every instruction
   (K12).
5. **Loud setup.** Every setup transaction prints its logs and panics on failure
   (`templates/README.md`, pattern 6). A setup smoke proves the builders before any action exists
   (K55, K56).
6. **Registry.** Register every account with kind and origin; print the map (K11, K20).
7. **Shadows.** Shadow mints with a token account per actor; realistic shadows of external
   accounts; attacker instances of permissionless initialize instructions (K18, K19).
8. **Slot range.** Count each pool. Choose `SLOT_RANGE` so that every candidate is reachable at
   75% honest (K21, K22).
9. **Initial state.** Put the protocol in a state where every action can succeed from the first
   iteration (for example, initial deposits into every pool).
10. **Ghost state.** Read the config, immutable fields and baselines after setup.

Related lessons: K57 and C7 (variant fixtures), K58 (byte constants from an earlier tier), K59
(deterministic smoke for a rare multi-step path).

### Step 10. Environment model

1. **Built-in programs.** Record which token program the runtime loads (R4). Load the version the
   target needs with `ctx.add_program` (K5).
2. **Coverage override.** If the harness loads more than 1 program, hide `FUZZ_PROGRAM_SO` around
   the other `add_program` calls (K7, C3).
3. **Clock and epoch.** If the program reads the epoch, set a custom epoch schedule without warmup,
   then write the full `Clock` at each time change (K6, R3).
4. **Native mint.** Build wSOL accounts with the rule in R5.
5. **Time per transaction.** Do not expect the clock to move by itself (R2). Add an environment
   action for time.

### Step 11. Actions

1. One action per program instruction. Each has a fuzzed signer index and fuzzed account slots
   (registry pick). The fee payer is the relayer.
2. One action per state change outside the program: donations, direct receipt-token transfers
   (K13). Add `.skip_account_mutation()` to these raw helper calls (K41, P5).
3. Amounts: `#[range]`, clamp, early return (K14). Slippage: `None` or expected plus or minus a
   small delta.
4. Config instructions: state a range for each field; include invalid values on purpose to test
   the rejections.
5. Multi-instruction transactions (flash loans): variants for full, open only, close only, and a
   fuzzed middle instruction. Relayer first in the batch (K15).
6. Environment actions: price sources and clock (`invariant-catalog.md`, 4.4). Make a destructive
   environment state rare in the selector (K61).
7. Each action stores in the fixture what the invariants need: `last_tag`, `last_kind`,
   `last_signer`, the pre-state for fee checks.
8. Each action returns `bool`. A `DEBUG` flag prints each failure with its error code.

Related lessons: K62 (loss paths), K63 (multi-op actions and `success`), K64 (price solver
bounds).

### Step 12. Invariants

1. Choose classes from `invariant-catalog.md`. For an economic protocol, include per-actor wealth
   and NAV per share (K26). Mark closed-set conservation as harness sanity (K25). Gap check after
   the doc list: K81.
2. Write all formulas in `model.rs` with unit tests from the doc examples (K84-K88).
3. Report through `report_violation` (K32, K33). Do not panic in an action or in the invariant
   (C6).
4. Give design cases and doc contradictions their own IDs (K34).
5. Record each tolerance and each skip with what it hides.
6. Compare sibling instructions and add an invariant for each asymmetric check you find, in the
   sealed note if the campaign is blind (K27).
7. Run `cargo check --features invariant_test` and `cargo test --lib`.
8. Dry run: `crucible run <lib> invariant_test --dry-run`. Expect the setup lines, the map, the
   pool sizes and no panic. A supplementary build needs its own dry run in each tier (K60).

Related lessons: K65-K71 (check design).

---

## Stage E. Run

### Step 13. Blocker loop (15 s runs)

Repeat until each action succeeds in a usable share of calls and edges stop growing between runs:

1. `crucible run <lib> invariant_test --release --timeout 15 > <log> 2>&1`. Use `DEBUG = true`.
2. Failure histogram: `grep '\[DEBUG\]' <log> | sort | uniq -c | sort -rn | head -20` (K30).
3. Decode each code with its instruction (K31, T6, T7). Anchor 2000-2999 = constraints, 3000+ =
   account errors, 6000+ = program errors.
4. Fix the dominant blocker in the harness. Do not widen an invariant bound or add a skip to
   silence a crash; report each unexpected crash to the user with its cause.
5. Read the final numbers from the `Timeout reached` line, not from the last pulse (K43, T1).

The first release build of a large harness can take about 16 minutes.

Related lessons: K89-K100 (reach lines and gate runs).

### Step 14. Expected-bucket check

1. List the buckets that the spec expects (design cases, known doc contradictions).
2. After the first runs, check that each one appears (K29). For canaries, see K66 and K59.
3. If one does not appear, print the computed values for its triggering action before you trust
   any result. This check found the first-wins defect (K33, C1).
4. Set `DEBUG = false` before the main run.

### Step 15. Sealed note (blind protocol)

1. Before the main run, write your own code suspicions to `sealed-suspicions.md`: file and line,
   the suspected property, and the invariant or action that would test it.
2. Header line: "Sealed: written before run N, not used in phase A."
3. Do not change the harness after this step until phase A ends, except for triage fixes that the
   user approves.
4. Use the note only in phase C, as labeled experiments.

### Step 16. Main run

1. Stateless mode for account-choice fuzzing (K23). Use its own crash directory under
   `<fuzz_dir>/crashes/`, so `tmin` can find it (T4). Save the corpus for coverage:

   `crucible run <lib> invariant_test --release --timeout 600 --corpus-out $H/corpus --crashes-out $H/crashes/runN > <log> 2>&1`
2. Keep the log. The violation messages are only there (T2).
3. Bucket list: `grep -o '{I[0-9a-z]* | [^}]*}' <log> | sort | uniq -c`.
4. Measure executions to the first finding of each bucket (T1), not wall time or exec/sec.
5. Optional: a `--stateful` run for bugs that need deep valid states (T5). See K101-K104 and
   S1-S3 for pool size, seeds and pool admission.

### Step 17. Probe pass

1. Run after the main run, in its own crash directory:

   `crucible run <lib> invariant_test --release --mutate-accounts --timeout 300 --crashes-out $H/crashes/probeN > <log> 2>&1`
2. Expect lower exec/sec and more edges (P6).
3. Requirements: each instruction succeeds sometimes; token accounts and their mints are in the
   instruction; authorities are in `.signers()` with a relayer as payer (K36, K38, P3).
4. Limits: coded shapes only, same-effects oracle, 1 transaction, 1 probe per (instruction, path)
   (K39).

---

## Stage F. Evaluate

### Step 18. Triage

1. List: `crucible show <lib> --crashes-dir <dir>`. Ignore `.crash_corpus` entries with "no
   metadata" (T3).
2. Replay one crash per bucket: `crucible show <lib> <crash_id> --replay`. Build the release
   binary first after a harness fix (K74, T8).
3. Minimize: `crucible tmin <lib> <crash_id> --release` (auto-detect under `crashes/`, T4). `tmin`
   overwrites the file in place.
4. Map addresses to roles with the setup map (K11). Map discriminators with the IDL
   (`instructions[].discriminator`).
5. Group buckets by (invariant, action). Buckets that differ only in slot relations are probably
   one cause (K34).
6. Probe findings: check each against the false-positive patterns (P4, K72). A mutation crash ID
   is (class, instruction), not an input (K35).
7. Classify each crash: candidate finding (a doc property that the program violates), design case,
   false positive (harness or engine artifact), or environment observation.
8. Record each crash with the minimal sequence, mechanism (file and line), size, and status.
9. Record harness defects that triage found, with their fixes.

### Step 19. Coverage

Use the 3-binary procedure (V1):

1. Build DWARF binaries with `CARGO_PROFILE_RELEASE_*` variables and `CARGO_TARGET_DIR=target-cov`:
   `cargo build-sbf --debug --tools-version v1.52 --sbf-out-dir $PWD/target-cov/deploy --manifest-path programs/<crate>/Cargo.toml`
   If the build fails on old locked dependencies, see K53 and B8.
2. Copy `target-cov/deploy/<lib>.so` to `cov/debug.so` and
   `target-cov/sbpf-solana-solana/release/<lib>.so` to `cov/symbols.so`. Keep `/target/` out of
   the symbols path (K75, V5).
3. Replay the main-run corpus:
   `crucible run <lib> invariant_test --release --coverage --corpus-in $PWD/fuzz/<lib>/corpus --program-so $PWD/fuzz/<lib>/cov/debug.so --program-id <ID> --symbols $PWD/fuzz/<lib>/cov/symbols.so --lcov-out $PWD/fuzz/<lib>/cov/coverage.lcov`
4. `lcov --extract cov/coverage.lcov '*/programs/<crate>/*' -o cov/program.lcov`, then
   `genhtml cov/program.lcov -o cov/html --legend`. If the harness loads more than 1 program,
   keep only the LCOV block of the target (K76, V6).
5. List files never reached (K44, V2). Map each uncovered block to a missing action, an unbuilt
   tier, or a rare state (K45).

If the replay fails in setup with `DeclaredProgramIdMismatch`, see C3.

Related lessons: K77 and K78 (focus corpus replays), K79 and V7 (union of 2 LCOV files).

### Step 20. Notes, run records and scoring

Note files per campaign:

| File | Content |
|------|---------|
| `00-journal.md` | Chronological log: date, step, decision, result |
| `01-target-intake.md` | Step 3 |
| `02-partitioning-large-targets.md` | Step 4 and tier results |
| `03-harness-design.md` | Each design decision with its reason; tolerances; skips; blind spots |
| `04-runs.md` | One run record per run |
| `05-findings.md` | Address map; each crash with triage; false positives; scoring table |
| `06-lessons.md` | New lessons: Observation, Rule, Input needed |
| `sealed-suspicions.md` | Step 15 |
| `specs/`, `plans/` | Steps 5-7 |

Run record template:

```
## Run N: <harness commit>, <mode/flags>, <duration>
- Command:
- Result: executions, exec/sec, corpus, edges, success %, crashes
- Findings: bucket or probe class, time and executions to first
- Failure histogram (top 5-10) with decoded codes
- Interpretation (mark hypotheses)
```

Scoring table (phase D), one row per report finding:

| Field | Content |
|-------|---------|
| ID, severity, title | From the report |
| Class | account validation / access control / arithmetic / economic / state machine / off-chain or other |
| Fuzzable? | yes / no (and why) |
| Detected in phase A? | yes / no; oracle (probe class or invariant ID); executions to first detection |
| Detected in phase C? | yes / no; which change; from a report hint? |
| Reason if missed | no action for the path / no invariant for the property / account not in pool / probe shape not covered / tier not built / other |

Also record every crash that is not in the report: new finding or false positive.
