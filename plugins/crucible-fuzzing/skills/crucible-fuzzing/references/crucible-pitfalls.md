# Crucible and runtime pitfalls

Tool and runtime facts that earlier Crucible campaigns found or used. Each fact has a version tag.
Check each fact again when a version changes.

## Version tags

| Tag | Meaning |
|-----|---------|
| `[crate]` | crates.io `crucible-test-context` 0.3.0-alpha.1 (linked by the harness). Path: `~/.cargo/registry/src/*/crucible-test-context-0.3.0-alpha.1/src/` |
| `[src]` | Local Crucible source 0.3.0-alpha.1 (`crucible/crates/`), from which the `crucible` CLI was built |
| `[docs]` | `crucible/docs/*.md` of the local source |
| `[litesvm]` | `litesvm` 0.15.2 crate (`~/.cargo/registry/src/*/litesvm-0.15.2/src/`) |
| `[anchor30]`, `[anchor31]` | Anchor 0.30.1 or 0.31.1 (program and IDL) |
| `[pt152]` | Solana platform-tools v1.52 |
| `[anchor29]` | Anchor 0.29.0 target program |
| `[sol117]` | Solana 1.17.x toolchain (`cargo-build-sbf` 1.17.34) for the target program |

Line numbers are for these exact versions. "Observed" means that a campaign observed the fact in
a run, a build or an experiment. Lesson IDs `Kn` point to `lessons.md`.

## Summary table

| ID | Pitfall | Tag | Lesson |
|----|---------|-----|--------|
| B1 | `avm use` does not always move the `anchor` link | observed | K2 |
| B2 | Old `proc-macro2` breaks the IDL step on new Rust | `[anchor30]`, observed | K2 |
| B3 | IDL-gen omits fixed-address accounts (0.31 IDL), not in 0.30 IDLs | `[anchor31]`, `[anchor30]` | K3 |
| B4 | The test function name must equal a Cargo feature | `[docs]` | |
| B5 | Anchor 0.29 writes a legacy IDL; `crucible-idl-gen` needs the new format | `[anchor29]`, observed | K49 |
| B6 | A fixture `.so` with `.symtab` fails to load (`Unknown symbol with index N`) | `[crate]`, observed | K51 |
| B7 | `cargo-build-sbf` 1.17 on arm64: C build scripts need the host compiler | `[sol117]`, observed | K52 |
| B8 | DWARF build with new platform tools fails on old `ahash` (`stdsimd`) | `[pt152]`, observed | K53 |
| R1 | Signature verification is off | `[crate]` | K38 |
| R2 | Slots do not advance per transaction in fuzz mode | `[crate]` | K6 |
| R3 | `warp_to_slot` does not move the epoch; default schedule has warmup | `[litesvm]` | K6 |
| R4 | p-token is the default token program; it rejects `SyncNative` with extra accounts | `[litesvm]`, observed | K5 |
| R5 | No native mint account; wSOL build rule | observed | |
| C1 | `record_violation` is first-wins per iteration | `[crate]`, `[src]` | K33 |
| C2 | `send_batch` uses the first unique signer as fee payer | `[crate]`, `[src]` | K15 |
| C3 | `FUZZ_PROGRAM_SO` scoping differs between CLI source and crate | `[crate]`, `[src]` | K7 |
| C4 | Invariant runs after every action; fixture is cloned per iteration | `[docs]` | K33 |
| C5 | `fuzz_assert_eq!` evaluates its arguments more than once | observed | |
| C6 | A panic in an action or in the invariant kills the fuzzer process | `[docs]`, observed | |
| C7 | Replacing the fixture SVM inside an iteration corrupts the template | `[crate]`, observed | K57 |
| P1 | Probe mechanism and run count | `[src]` | K36 |
| P2 | Mutation crash ID = class + program + discriminator prefix | `[crate]`, `[docs]` | K35, K41 |
| P3 | CC-4 skips the fee payer | `[crate]` | K38 |
| P4 | CC-3 and CC-13 false-positive patterns | `[docs]`, observed | K41 |
| P5 | `skip_account_mutation` excludes an instruction silently | `[crate]`, `[docs]` | K41 |
| P6 | Probe pass costs throughput and adds error-path edges | observed | K36 |
| T1 | `FUZZ_PULSE` is event-driven | observed | K43 |
| T2 | Invariant crashes are named by input hash; message only in the log | `[src]`, observed | K32 |
| T3 | `--crashes-out` gives a flat layout; `show` lists raw inputs | observed | |
| T4 | `tmin` reads only `<fuzz_dir>/crashes/<test>/` and overwrites in place | `[src]`, `[docs]` | K35 |
| T5 | Stateful mode output differs; docs disagree on `--max-depth` | `[docs]`, observed | K23 |
| T6 | Anchor error codes restart at 6000 per enum | observed | K31 |
| T7 | An error enum with an explicit first value shifts every code | observed | K73 |
| T8 | `crucible show --replay` uses the release binary that exists; it does not build | observed | K74 |
| S1 | `--pool-size` is per worker; the default pool needs tens of GB | observed | K103 |
| S2 | `--corpus-in` on a stateful run gives false buckets | observed | K104 |
| S3 | A post state enters the stateful pool only with field novelty or new edges | observed | K101 |
| V1 | DWARF build requirements and symbol file paths | `[docs]`, `[pt152]`, observed | |
| V2 | LCOV denominator excludes files never reached | observed | K44 |
| V3 | Second program in LCOV can have a malformed record | observed | |
| V4 | Anchor-generated functions map to the `#[program]` line | observed | |
| V5 | `--symbols` path with `/target/` drops source files of a build in another folder | `[crate]`, observed | K75 |
| V6 | The `--symbols` DWARF map is attached to every loaded program | `[crate]`, observed | K76 |
| V7 | `BRDA` side order can differ between 2 replays of 1 binary | observed | K79 |

---

## B. Build and toolchain

### B1. `avm use` does not always move the `anchor` link (observed)

- Fact: `avm use 0.30.1` printed "Now using anchor version 0.30.1", but `~/.avm/bin/anchor` still
  pointed to `anchor-0.31.1`. `anchor --version` showed 0.31.1.
- Action: run `anchor --version` after each `avm use`.
- Observed effect: none on the `.so`, because the program code used the locked `anchor-lang`
  0.30.1.

### B2. Old `proc-macro2` breaks the IDL step `[anchor30]` (observed)

- Fact: `proc-macro2` 1.0.86 calls `proc_macro::Span::source_file`, which host Rust 1.86 does not
  have (E0599). `cargo build-sbf` succeeded; only the IDL step failed.
- Action: use the committed IDL after a check of its instruction list against `lib.rs`.

### B3. IDL-gen and fixed-address accounts `[anchor31]` `[anchor30]`

- Fact: `crucible_idl_gen::declare_fuzz_program!` generates no struct field for an IDL account that
  has an `address`. Anchor 0.31 IDLs give `system_program` and similar accounts an `address`.
- Fact: Anchor 0.30 IDLs (spec `0.1.0`) have no `address` on program and sysvar accounts. The
  generated structs then contain `system_program`, `token_program`, `associated_token_program`,
  `rent`, `instructions`, `clock`, and the harness must set them.

### B4. The test function name must equal a Cargo feature `[docs]`

- Fact: the function name `invariant_test` must equal a feature name in the harness `Cargo.toml`.
  The generated `main` exists only with that feature, so compile checks use
  `cargo check --features invariant_test`.

### B5. Anchor 0.29 IDL path `[anchor29]` (observed)

- Fact: `crucible-idl-gen` reads the new (0.30+) IDL format. An Anchor 0.29 program has only a
  legacy IDL.
- Fact: `anchor-0.29.0 idl build -p` is not a valid command. `anchor-0.29.0 build -p <name>
  --no-docs` writes the legacy IDL. `anchor-0.31.1 idl convert` converts it after
  `metadata.address` is added to the legacy JSON.
- Fact: instruction arguments with types from external crates did not block the IDL builders.
  Setup builders with enum or external arguments can use a raw instruction (`raw_call`).

### B6. Fixture `.so` with `.symtab` fails to load `[crate]` (observed)

- Fact: Crucible turns on the sbpf debug features, so `enable_symbol_and_section_labels` is true.
  solana-sbpf 0.21.0 reads each `.symtab` function name with a 64-byte limit
  (`elf_parser/mod.rs:13`, `elf.rs:1227-1241`). A long mangled Rust name fails the load with
  `ProgramLoad("Unknown symbol with index N")`.
- Action: check each fixture with `llvm-readelf -S`. Load a copy without `.symtab` and `.strtab`
  (`llvm-objcopy --remove-section=.symtab --remove-section=.strtab`). `cargo build-sbf` output is
  stripped, so the target program itself loads.

### B7. Solana 1.17 `cargo-build-sbf` on arm64 `[sol117]` (observed)

- Fact: the `cc` crate picks the platform-tools clang for host build scripts, which has no macOS
  headers (`assert.h` not found, seen in `blake3` 1.5.0).
- Action: set `CC_aarch64_apple_darwin=/usr/bin/clang AR_aarch64_apple_darwin=/usr/bin/ar`. The
  SBF output hash does not change.
- Fact: host builds of programs with `u128` size assertions can fail on arm64 with old Rust
  (16-byte against 8-byte alignment). Native host commands then need `--target
  x86_64-apple-darwin`.

### B8. DWARF coverage build and old `ahash` `[pt152]` (observed)

- Fact: platform-tools v1.51+ rustc reports a dev channel. `version_check` then enables the
  removed `stdsimd` feature in `ahash` below 0.7.8 and 0.8.11, and the build fails.
- Fact: `-Zallow-features` in `RUSTFLAGS` breaks the host `cargo metadata`; in the target flags it
  did not stop the build script.
- Action: plan coverage in a workspace copy with patch-level bumps, or as optional (fix not
  tested).

---

## R. Runtime (LiteSVM)

### R1. Signature verification is off `[crate]`

- Fact: `TestContext` builds the SVM with `with_sigverify(false)` (`[crate]` `lib.rs:1867`). In this
  mode the builder writes dummy signatures (`instruction_builder.rs:135-141`).
- Effect: only the `is_signer` flags in the message matter. A harness can "sign" with any keypair.
  The CC-4 `SuppliedSigner` probe uses this (P3).

### R2. Slots do not advance per transaction in fuzz mode `[crate]`

- Fact: nothing in `TestContext` or LiteSVM advances the slot per transaction. Only
  `warp_to_slot` and `advance_slots` change it (`[crate]` `lib.rs:2525-2534`). Thus prices and
  time change only through environment actions.
- Status: recorded from a source reading; not tested separately.

### R3. `warp_to_slot` does not move the epoch `[litesvm]`

- Fact: `warp_to_slot` sets only `clock.slot` (`[litesvm]` `lib.rs:1767-1771`).
- Fact: LiteSVM sets `EpochSchedule::default()` (`[litesvm]` `lib.rs:596`). The default schedule has
  warmup (epoch 0 = 32 slots).
- Fix: `ctx.set_sysvar(&EpochSchedule::custom(N, N, false))` in setup, then write the full `Clock`
  (slot, `epoch = slot / N`, `leader_schedule_epoch`) after each `warp_to_slot`. Template:
  `templates/README.md`, "chain.rs".

### R4. p-token is the default SPL Token program `[litesvm]` (observed)

- Fact: `load_default_programs` loads the Pinocchio token program (p-token) at
  the SPL Token program ID (`spl_token::ID`) when the feature
  `replace_spl_token_with_p_token` is active, else SPL Token 3.5.0 (`[litesvm]`
  `programs/mod.rs:13-30`). Crucible enables all features by default
  (`[docs]` `writing-tests.md:36`, "all-features-enabled policy").
- Fact (observed in an experiment): p-token rejects `SyncNative` with more than 1 account
  (`InvalidArgument`). With 1 account it succeeds.
- Fix: `ctx.add_program(&TOKEN_PROGRAM, "programs/spl_token-3.5.0.so")` in setup. The binary is in
  the `litesvm-0.15.2` crate (`src/programs/elf/spl_token-3.5.0.so`).

### R5. LiteSVM has no native mint account (observed)

- Fact: LiteSVM does not create the native mint account. A program that checks only the `mint`
  field of a wSOL token account needs no mint account.
- Build rule for a wSOL account: `.mint(NATIVE_MINT).is_native(Some(rent)).amount(x).lamports(rent + x)`.
  If `lamports` is only the rent, the first `sync_native` resets `amount`.

---

## C. `crucible-test-context` API

### C1. `record_violation` is first-wins per iteration `[crate]` `[src]`

- Fact: `record_violation` stores the message only if no violation is stored yet
  (`lib.rs:1122-1133`, same lines in `[crate]` and `[src]`). `record_mutation_violation` has the same
  rule (`lib.rs:1138-1149`).
- Effect: the second violation of an iteration is dropped. A bucketing function that marks a bucket
  as seen before this check loses that bucket for the whole run.
- Fix: the `report_violation` in `templates/account_fuzzing.rs` with the `violation_recorded`
  fixture flag. Do not reset the flag in the invariant function.

### C2. `send_batch` uses the first unique signer as fee payer `[crate]` `[src]`

- Fact: `send_batch` deduplicates the signers of all queued instructions in order and takes the
  first as fee payer. `.fee_payer()` on a queued call is not used
  (`[src]` `lib.rs:2865-2889`; `[crate]` `lib.rs:2836-2860`).
- Fix: relayer first in `.signers()` of the first `add_transaction`.

### C3. `FUZZ_PROGRAM_SO` scoping differs between CLI source and crate `[crate]` `[src]`

- Fact `[src]`: `program_so_override` applies `FUZZ_PROGRAM_SO` only to the program named by
  `FUZZ_PROGRAM_ID` when that variable is set (`lib.rs:1818-1845`).
- Fact `[crate]`: `add_program` applies `FUZZ_PROGRAM_SO` to every call, with no `FUZZ_PROGRAM_ID`
  check (`lib.rs:1997`, `2019-2031`).
- Fact `[docs]`: `coverage.md:306` describes the `[src]` behavior.
- Effect: in coverage mode, a harness that loads a second program (for example SPL Token) gets the
  target binary at that address. Observed error: `DeclaredProgramIdMismatch`.
- Fix: in `setup()`, remove `FUZZ_PROGRAM_SO` from the environment around the other
  `add_program` calls, then restore it. Excerpt: `templates/README.md`, "Environment model in
  setup".

### C4. Invariant after every action; fixture cloned per iteration `[docs]`

- Fact: the `#[invariant_test]` function runs after each action of the sequence
  (`writing-tests.md:171`). The fixture is `#[derive(Clone)]` and is cloned from the post-setup
  state for each iteration (`writing-tests.md:6`).
- Effect: ghost fields reset at the start of each iteration. A field that must live for the whole
  iteration must not be reset in the invariant function (C1). The invariant function gets
  `&mut fixture`, so it can keep its own previous values (baselines).

### C5. `fuzz_assert_eq!` evaluates its arguments more than once (observed)

- Fact: the macro evaluates `a` and `b` more than once. Pass plain values, not calls with side
  effects.

### C6. A panic kills the whole fuzzer process `[docs]` (observed)

- Fact: a panic in an `action_*` method or in the invariant function kills the whole fuzzer
  process, not one iteration.
- Rule: use `fuzz_assert_*` or `record_violation` / `report_violation`. Never call `unwrap()` on a
  read of a fuzzed account slot (`templates/README.md`, pattern 5).

### C7. Replacing the fixture SVM corrupts the template `[crate]` (observed)

- Fact: Crucible swaps the real SVM into the fixture for each iteration and restores only the
  dirty accounts from the template snapshot after it (`crucible-fuzz-macro` `singlecore.rs:115-145`).
- Fact: a fixture object replaced inside an iteration goes back into the template slot with mixed
  accounts. Observed result: many false buckets in a 15 s run.
- Action: write the changed accounts with `write_account` into the running SVM; keep variant
  baselines in the fixture.

---

## P. Probe engine (`--mutate-accounts`)

### P1. Mechanism and run count `[src]`

1. `--mutate-accounts` sets `FUZZ_MUTATE_ACCOUNTS`. Each `.send()` then probes before the real
   transaction.
2. Baseline: the engine clones the SVM and runs the unchanged transaction. If it fails, no probe.
3. Dedup: it hashes the baseline edge trace. Each (instruction, executed path) is probed once.
4. Probes: for each strategy it clones the baseline SVM, plants decoy accounts and rewrites the
   account metas.
5. Oracle: the mutated transaction succeeds, and all other accounts have the same post-state as the
   baseline.
6. Report: `record_mutation_violation`, same channel as invariants (C1).

- Run count for one probed send: 1 baseline + 1 per probe candidate + 1 real run.
- Code: `crucible-test-context/src/account_mutation.rs`, `lib.rs`, `program_builder.rs`.

### P2. Mutation crash ID `[crate]` `[docs]`

- Fact: finding ID = `{class}:{program}:{discriminator hex}` (`[crate]`
  `account_mutation.rs:511-519`). The account role is not part of it. The crash file name is a hash
  of this ID (`crucible-fuzz-macro/src/singlecore.rs`).
- Fact: the discriminator length is the IDL-registered length (8 for Anchor), clamped to the data
  length, with 8 as fallback (`[crate]` `account_mutation.rs:389-398`; `[docs]`
  `constraint-check-engine.md:6-20`). The same length applies to every instruction of the
  harness, also to non-target instructions.
- Effect 1: a later run that finds the same (class, instruction) overwrites the crash file (K35).
- Effect 2: a System `Transfer` has a 4-byte tag, then the amount. Its first 8 bytes contain amount
  bytes, so each distinct amount gives a new finding ID (many CC-3 IDs in one observed run, K41).
- Effect 3: 2 bugs of the same class in one instruction collapse into 1 finding. The message names
  the first account (`[docs]` `constraint-check-engine.md:17-20`).

### P3. CC-4 skips the fee payer `[crate]`

- Fact: `SignerStrategy::probe` skips every candidate equal to the payer (`[crate]`
  `account_mutation.rs:825-843`). Candidates are metas with `is_signer` and supplied keypairs whose
  key is in the instruction (`SuppliedSigner`, `account_mutation.rs:884+`).
- Effect: if the authority is the fee payer, a missing signer check is never probed.
- Fix: relayer as fee payer, authority in `.signers()` (K38).
- Docs: `[docs]` `constraint-check-engine.md:68` ("payer-as-authority cases are intentionally
  conservative").

### P4. CC-3 and CC-13 false-positive patterns `[docs]` (observed)

| Class | Pattern | Why it is a false positive |
|-------|---------|----------------------------|
| CC-3 pda-spoof | Harness System `Transfer` into a PDA | Not a target instruction; a transfer to any account succeeds |
| CC-13 forwarded-account | Lamport-only PDA (fixed by seeds) forwarded to System | Owner and data at that address cannot change on chain |
| CC-13 forwarded-account | Signer wallet forwarded to Token as transfer authority | Owner and data of a signer wallet do not matter to Token |
| CC-13 forwarded-account | By-design arbitrary transfer recipient (fee account) forwarded to System | The docs name this as the residual false positive |

- `[docs]` `constraint-check-engine.md:67` (CC-3: "Lamport-only PDA slots may be intentionally
  flexible") and `:82` (CC-13 residual false positive). Other known patterns: CC-1 on
  address-pinned accounts, CC-9.5 on SPL non-authority windows (`:90-98`).
- Observed: the signer-wallet row repeats on other targets (for example a wallet forwarded to
  Token-2022 as transfer authority). Classify it without a replay (lesson K72).

### P5. `skip_account_mutation` `[crate]` `[docs]`

- Fact: `.skip_account_mutation()` on a builder (`[crate]` `instruction_builder.rs:30`) or
  `ctx.skip_account_mutation_for(&ix)` (`[crate]` `lib.rs:2291`) excludes an instruction type from
  all probe classes. The key is (program, discriminator). It also covers `send_batch`. It is silent.
- Use it for harness helper instructions only (K41). Do not use it to hide target findings.
- Docs: `[docs]` `constraint-check-engine.md:100-121`.

### P6. Probe pass cost (observed)

- Fact: in one observed probe pass, exec/sec fell by about 30%, and the edge count rose (the
  probes reach new error paths).
- `[docs]` `constraint-check-engine.md:32-35`: run it after the corpus plateaus; it is a
  thoroughness pass, not a long-run mode.

---

## T. Triage and CLI

### T1. `FUZZ_PULSE` is event-driven (observed)

- Fact: a pulse line appears when the corpus grows. The end of a run is the line
  `[FUZZ] Timeout reached (<n>s)`.
- "Executions to first finding" = `executions` of the last pulse before the `[FUZZ_FINDING]` line.

### T2. Invariant crash names and messages `[src]` (observed)

- Fact: an invariant crash file is `crash_<hash(input bytes)>`. Each new input that breaks the
  invariant is a new file. No dedup.
- Fact: the violation message is not in the crash metadata. It is only in the run log
  (`[FUZZ_FINDING]`). Keep every run log.

### T3. `--crashes-out` gives a flat layout (observed)

- Fact: with `--crashes-out`, the directory is flat (`test: (flat)`). `crucible show --crashes-dir`
  also lists the raw inputs in `.crash_corpus/` as crashes with "no metadata". Ignore those.

### T4. `tmin` crash directory `[src]` `[docs]`

- Fact: `crucible tmin` has no `--crashes-dir` flag. It reads `<fuzz_dir>/crashes/<test_name>/`
  (`[src]` `crucible-fuzz-cli/src/lib.rs:1766-1776`).
- Fact: if the second argument starts with `crash_`, `tmin` searches every subdirectory of
  `<fuzz_dir>/crashes/` for it (`lib.rs:1731-1758`). Thus a crash in `crashes/runN/` is found with
  `crucible tmin <program> crash_<id> --release`, or with `runN` as the test name.
- Fact: a crash directory outside `<fuzz_dir>/crashes/` is not reachable by `tmin`. Put
  `--crashes-out` directories under `<fuzz_dir>/crashes/`.
- Fact: `tmin` overwrites the crash file in place (`[docs]` `crash-analysis.md:33`,
  `cli-reference.md:146`).

### T5. Stateful mode output and `--max-depth` `[docs]` (observed)

- Fact: the stateful pulse is `[mm:ss] iter: N, iter/sec: N, pool: N/250k, ...`. Findings print
  `at iteration N`.
- Fact: the docs disagree on the `--max-depth` default: 15 (`cli-reference.md:36`) and 100
  (`cli-reference.md:177`). Set it explicitly.

### T6. Anchor error codes restart at 6000 per enum (observed)

- Fact: each `#[error_code]` enum starts at 6000. Decode a code together with its instruction, or
  parse `Error Code: <Name>` in the logs (K31).

### T7. Error enum with an explicit first value (observed)

- Fact: if an `#[error_code]` enum starts with `Name = 1000`, the codes are `6000 + 1000 +
  position`. A line filter for `Name,` skips the first variant, and every code is then 1 too low.
- Action: parse `= N` too; assert the codes that the harness uses in a unit test (lesson K73).

### T8. `crucible show --replay` does not build (observed)

- Fact: `crucible show` replays with the release binary that exists. `cargo check` and `cargo test
  --lib` do not write `target/release/<test>`. A replay after a fix on the old binary reproduces
  the old bucket.
- Action: `cargo build --release --features <test>` before each replay; compare the binary time
  with the source time (lesson K74).

---

## S. Stateful mode

### S1. Pool size and memory (observed)

- Fact: the default pool holds about 256,000 states. At the default pool size, 1 stateful run of a
  large harness used about 31 GB (up to 48 GB when it ran alone). Parallel runs above the machine
  RAM caused a macOS watchdog panic.
- Fact: `--pool-size` is a limit per worker. `-j 6 --pool-size 40000` reached about 211,000 states
  and 9.8 GB per run.
- Action: compute runs x workers x pool size x bytes per state against the RAM before parallel
  stateful runs. A short test does not fill the pool (lesson K103).

### S2. `--corpus-in` on a stateful run (observed)

- Fact: a stateful run seeded with an earlier stateful corpus gave false buckets on many checks
  within 120 s. The same binary without the seed gave 0 buckets.
- Action: start stateful runs with an empty corpus. Use a stateful corpus only for coverage replay
  (lesson K104).

### S3. Pool admission (observed)

- Fact: a post state enters the pool only with field novelty or new edge coverage. A step that
  changes 1 small field is seldom saved, so a chain that needs that step restarts from another
  state.
- Fact: a pool state holds no violation, so the stateful bucket list is a lower bound. Judge
  buckets on the stateless run and long chains on the stateful run (lessons K101, K102).

---

## V. Coverage

### V1. DWARF build requirements and symbol paths `[docs]` `[pt152]` (observed)

- Requirements: `opt-level = 1`, `debug = 2`, `strip = false`, `cargo build-sbf --debug`,
  platform-tools v1.51 or later (`[docs]` `coverage.md`).
- Observed procedure: set the profile with `CARGO_PROFILE_RELEASE_*` variables and a separate
  `CARGO_TARGET_DIR=target-cov`. Then the program `Cargo.toml` and the fuzzing `.so` stay unchanged.
  `CARGO_PROFILE_RELEASE_LTO=false` was also set (effect on DWARF accuracy: assumption, not
  verified).
- Paths `[pt152]`: stripped binary `target-cov/deploy/<name>.so`; DWARF binary
  `target-cov/sbpf-solana-solana/release/<name>.so`. One observed build also wrote
  `target-cov/deploy/<name>.debug`.
- Executed binary and DWARF binary must come from the same build.
- Use absolute paths for `--program-so`, `--symbols`, `--lcov-out`, `--corpus-in`. Limit
  `--program-so` to the target with `--program-id` and see C3.

### V2. LCOV denominator (observed)

- Fact: files with no hit in the corpus replay are not in the LCOV file. Setup code is not part of
  the replay, so init handlers are not measured (K44).

### V3. Malformed record of a second program (observed)

- Fact: an observed LCOV file had a second, bytecode-level program record (probably SPL Token)
  with `LF:0` and a nonzero `LH`. `lcov --extract '*/programs/<crate>/*'` removes it.

### V4. Anchor-generated functions map to the `#[program]` line (observed)

- Fact: DWARF assigns all `#[program]`-generated functions (dispatch, IDL instructions) to the
  macro line. genhtml counts them as aliases, so function coverage of `lib.rs` looks low. In an
  observed program, most unhit functions were Anchor IDL instructions.

### V5. `--symbols` path and `/target/` `[crate]` (observed)

- Fact: `is_user_source` (`crucible-test-context` 0.3.0-alpha.1, `src/coverage/lcov.rs`) takes the
  text before `/target/` in `FUZZ_SYMBOLS` as the workspace root and drops other absolute paths. A
  DWARF build in a scratch copy then gives `Source-level coverage: 0 source files`.
- Action: put the symbols file at a path without `/target/` (for example `.cov/symbols.so`).
  Lesson K75.

### V6. DWARF map on every program `[crate]` (observed)

- Fact: the generated `init_dwarf_maps` (`crucible-fuzz-macro` 0.3.0-alpha.1, `codegen.rs`) inserts
  the same map for every loaded program. PCs of fixture programs map to false target lines. The log
  has 1 `Source-level coverage: N source files` line per program.
- Action: keep only the LCOV block of the target program (most files, target handlers). Lesson
  K76.

### V7. `BRDA` side order (observed)

- Fact: 2 replays of the same binary can order the outcome sides of a line differently (observed
  on derive-macro lines). A merge by position overstates the branch union.
- Action: check the side order per shared line before a `BRDA` union, or report the line union
  only (lesson K79).
