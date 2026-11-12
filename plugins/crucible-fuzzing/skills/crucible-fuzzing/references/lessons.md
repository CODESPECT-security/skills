# Lessons (K1-K104)

This file holds 104 lessons for Crucible fuzzing of Solana programs, grouped by topic. The
lessons come from 3 audit campaigns on Anchor programs of different size.

Format of each lesson:

- **Rule.** What to do (generic, imperative).
- **Why.** The cause or the mechanism.
- **Input needed.** The data that you must read before you apply the rule.
- **Status.** The number of targets that showed the lesson, and if the lesson is a hypothesis.

Pitfall IDs (B1, C1, P4, R4, ...) refer to `crucible-pitfalls.md`. Invariant classes refer to
`invariant-catalog.md`.

## Index

| Topic | Lessons |
|-------|---------|
| 1. Environment and toolchain | K1-K7, K49-K54 |
| 2. Setup and fixture | K8-K11, K55-K60 |
| 3. Actions and inputs | K12-K15, K61-K64 |
| 4. Account choice and shadows | K16-K24 |
| 5. Invariants | K25-K29, K65-K71 |
| 6. Crash bucketing and triage | K30-K35, K72-K74 |
| 7. Probe engine (`--mutate-accounts`) | K36-K41 |
| 8. Coverage | K42-K45, K75-K79 |
| 9. Process and review | K46-K48, K80-K83 |
| 10. Harness models and unit tests | K84-K88 |
| 11. Reach lines and gate runs | K89-K101 |
| 12. Stateful and long runs | K102-K104 |

---

## 1. Environment and toolchain

### K1. Use the `[lib] name` as the Crucible program name

- **Rule.** Use the `[lib] name` of the program `Cargo.toml` as the Crucible program name. Do not
  use the crate folder name.
- **Why.** The `.so` and the IDL file names come from the lib name. The generated harness loads
  `../../target/deploy/<program_name>.so`.
- **Input needed.** Program `Cargo.toml`; file list of `target/deploy/`.
- **Status.** Seen on 2 targets.

### K2. Check the tool versions that the build uses

- **Rule.** After `avm use <version>`, run `anchor --version` and compare. If only the IDL step of
  `anchor build` fails, use the committed IDL. Before that, compare its instruction list with the
  program `lib.rs`.
- **Why.** `avm use` can report success while the `anchor` link still points to another version.
  Old `proc-macro2` versions do not compile on new host compilers. The `.so` can still build.
- **Input needed.** `anchor --version`; build log; `target/idl/*.json`; `lib.rs`.
- **Status.** Seen on 1 target.

### K3. Check which IDL accounts the generated structs contain

- **Rule.** Before you write `accounts::X { .. }`, read the generated struct or the IDL. Anchor
  0.31 IDLs give program and sysvar accounts a fixed `address`, and `crucible-idl-gen` omits these
  fields. Anchor 0.30 IDLs have no `address` on them, so the fields exist and the harness must set
  them.
- **Why.** The wrong assumption gives a compile error (`has no field named system_program`) or a
  missing field.
- **Input needed.** IDL `instructions[].accounts[].address`; IDL spec version.
- **Status.** Seen on 2 targets.

### K4. Match the token program of the account builders

- **Rule.** Default to SPL Token. Pass `crucible_fuzzer::anchor_spl::token::ID` as
  `token_program`. Use Token-2022 only if the program requires it.
- **Why.** `ctx.create_mint()` and `ctx.create_token_account()` create SPL Token accounts. The
  harness must pass the same program ID.
- **Input needed.** `token_program` type in the source (`Program<Token>` or
  `Interface<TokenInterface>`).
- **Status.** Seen on 1 target. Builder support for Token-2022 accounts is not verified.

### K5. The built-in programs of the test runtime are part of the environment model

- **Rule.** Record which version of each built-in program the runtime loads. If setup fails inside
  a built-in program, compare it with the version that the target was written for. If the target
  needs another version, load it in `setup()` with `ctx.add_program`. Record the difference as an
  environment observation.
- **Why.** A runtime default can differ from the cluster that the program targets. The failure
  then looks like a program bug or a harness bug. For example, LiteSVM 0.15.2 loads p-token at the
  SPL Token address (pitfall R4).
- **Input needed.** `litesvm/src/programs/mod.rs`; target `Cargo.lock` (`spl-token` version);
  program logs of the failed CPI.
- **Status.** Seen on 1 target.

### K6. Control the clock and the epoch together

- **Rule.** If the program reads `Clock.epoch` or `EpochSchedule`, set a custom epoch schedule
  without warmup in `setup()`. Write the full `Clock` (slot, epoch, leader schedule epoch) at each
  time change.
- **Why.** `warp_to_slot` sets only `clock.slot`. The default epoch schedule has warmup. A program
  that computes epoch progress from slot and epoch then sees random values (pitfall R3).
- **Input needed.** Program uses of `Clock::get`, `EpochSchedule::get`, `clock.epoch`.
- **Status.** Seen on 1 target.

### K7. The CLI and the harness crates can be different Crucible versions

- **Rule.** If the harness loads more than 1 program, read the `add_program` override logic of the
  linked crate version. Do not trust the CLI docs for it. Hide `FUZZ_PROGRAM_SO` around the
  `add_program` calls of other programs, or pin the harness crates to the CLI source.
- **Why.** The CLI from source limits `--program-so` to the `--program-id` program. The crates.io crate
  applies the override to every `add_program` call (pitfall C3). A harness that loads 1 program
  does not show the problem.
- **Input needed.** Harness `Cargo.lock`; crate source of `add_program`.
- **Status.** Seen on 1 target.

### K49. Anchor below 0.30: plan the IDL path at intake

- **Rule.** For an Anchor 0.29 or older target, record at intake that there is no 0.30 spec IDL.
  Plan: build the legacy IDL with the matching CLI (`anchor-0.29.0 build -p <program> --no-docs`;
  `anchor idl build -p` is not a valid command in 0.29), then run `anchor idl convert` with a newer
  CLI. Add `metadata.address` to the legacy IDL before the conversion. Check whether instruction
  arguments use types from external crates; plan a `raw_call` fallback for them.
- **Why.** `crucible-idl-gen` needs the new IDL format. A legacy parser may not resolve external
  argument types.
- **Input needed.** `anchor-lang` version; instruction argument types.
- **Status.** Seen on 1 target. The external argument types did not need the `raw_call` fallback.

### K50. Check relative paths in build configuration at the start of each session

- **Rule.** At the start of a session, check that the paths in local build configuration
  (`.cargo/config.toml` patches, scripts) still exist. Record a missing path as a blocker before
  any build step.
- **Why.** Folders around the target can move between sessions. A relative `[patch]` path then
  breaks every build with an unclear fetch error.
- **Input needed.** Local build configuration files.
- **Status.** Seen on 1 target.

### K51. Strip `.symtab` from fixture programs before Crucible loads them

- **Rule.** Before `add_program` of a fixture `.so` that you did not build, check it with
  `llvm-readelf -S`. If it has a `.symtab`, load a copy without `.symtab` and `.strtab`
  (`llvm-objcopy --remove-section=.symtab --remove-section=.strtab`). Keep the original; print the
  SHA-256 of both in setup.
- **Why.** Crucible turns on the sbpf debug features, so `enable_symbol_and_section_labels` is
  true. solana-sbpf 0.21.0 then reads every `.symtab` function name with a 64-byte limit. A long
  mangled Rust name fails the load with `ProgramLoad("Unknown symbol with index N")`.
  `cargo build-sbf` output is stripped, so the target program loads and only the fixture fails.
- **Input needed.** Section list of each fixture `.so`.
- **Status.** Seen on 1 target.

### K52. Old Solana toolchains on Apple Silicon need the host C compiler for build scripts

- **Rule.** When `cargo-build-sbf` 1.17.x fails in a C build script (`assert.h` not found), set
  `CC_aarch64_apple_darwin=/usr/bin/clang AR_aarch64_apple_darwin=/usr/bin/ar`. Record the
  variables next to the build command.
- **Why.** The `cc` crate picks the platform-tools clang for host build scripts. That clang has no
  macOS headers. The SBF output does not change.
- **Input needed.** Host architecture; toolchain version.
- **Status.** Seen on 1 target.

### K53. A DWARF coverage build with new platform tools can fail on old locked dependencies

- **Rule.** Before the coverage step, check the lockfile for dependencies that turn on removed
  nightly features through `version_check` (for example `ahash` below 0.7.8 and 0.8.11 with
  `stdsimd`). Plan the coverage build in a copy of the workspace with patch-level bumps, or plan
  coverage as optional.
- **Why.** Platform tools v1.51 or later are necessary for DWARF line data (pitfall V1). Their
  rustc reports a dev channel, so `version_check` enables `stdsimd`, which no longer exists.
  `-Zallow-features` in `RUSTFLAGS` breaks the host `cargo metadata`; in the target flags it did
  not stop the build script.
- **Input needed.** Lockfile versions of `ahash` and other `version_check` users.
- **Status.** Seen on 1 target. The copy-and-bump fix is a hypothesis, not tested by a run.

### K54. Check that the fixture program has each instruction of the spec before the plan

- **Rule.** Before the plan, run each external instruction that the spec uses once against the
  fixture binary (a dry-run smoke step or a 1-instruction test). Write the result in the facts
  file. Do not take the instruction set from the source copy.
- **Why.** The fixture binary can be an older build than the source that the facts file reads. A
  missing instruction removes actions, reach lines and pass-rule items late, after the spec and
  the plan are approved.
- **Input needed.** The fixture `.so` files and their build notes.
- **Status.** Seen on 1 target.

---

## 2. Setup and fixture

### K8. Order setup by account creation dependencies

- **Rule.** Build a graph of which instruction creates each account (`init` in the source, or
  `writable` + `pda` in the IDL). Create accounts and call setup instructions in topological order.
- **Why.** Some harness accounts need accounts that a program instruction creates.
- **Input needed.** `#[account(init, ...)]` constraints; IDL account lists.
- **Status.** Seen on 1 target.

### K9. Derive PDAs from the IDL seeds

- **Rule.** Derive PDAs with `Pubkey::find_program_address` from the IDL seeds. Read the Rust
  source only when a seed is an instruction argument.
- **Why.** The IDL lists `const` and `account` seeds, but it does not show how arguments are
  encoded.
- **Input needed.** IDL `instructions[].accounts[].pda.seeds`.
- **Status.** Seen on 1 target.

### K10. Use deterministic keypairs

- **Rule.** Create every keypair with `Keypair::new_from_array([tag; 32])`, with one unique tag
  for each role. Keep a tag table in the notes.
- **Why.** Addresses stay the same across runs, replays and crash files.
- **Input needed.** None.
- **Status.** Seen on 2 targets.

### K11. Print a role-to-address map in setup

- **Rule.** Print a `role -> address` table once in `setup()`. In triage, replace each address in
  a finding with its role.
- **Why.** Probe findings name accounts only by base58 address.
- **Input needed.** Fixture fields; registry.
- **Status.** Seen on 1 target.

### K55. A setup smoke on fixture clones proves the builders before any action exists

- **Rule.** Before the first new action, run a setup smoke on clones of the fixture. It must
  build every new account and pin the K84 values on the chain.
- **Why.** A builder error found after the actions exist looks like an action bug. The smoke
  isolates the setup.
- **Input needed.** The list of the new accounts and the K84 values that the setup must hold.
- **Status.** Seen on 1 target.

### K56. A setup smoke needs every builder that it sends, so the builders belong to the smoke task

- **Rule.** Put each instruction builder that a setup smoke sends into the same task as the
  smoke. Do not leave a builder for a later action task.
- **Why.** A smoke that sends a builder from a later task does not compile or does not run at the
  end of its own task. The task gate cannot pass.
- **Input needed.** The list of builders that each smoke step sends.
- **Status.** Seen on 1 target.

### K57. A variant fixture must write its accounts into the running SVM, not replace the fixture

- **Rule.** To switch a fixture variant inside an iteration, write the changed accounts into the
  running SVM with `write_account` and keep the variant baselines in the fixture. Do not replace
  the fixture object.
- **Why.** Crucible swaps the real SVM into the fixture for each iteration and restores only the
  dirty accounts from the template snapshot after it. A replaced fixture SVM goes back into the
  template slot with mixed accounts.
- **Input needed.** The iteration loop of `crucible-fuzz-macro` (`singlecore.rs`).
- **Status.** Seen on 1 target.

### K58. K84 byte constants from an earlier phase hide later setup steps

- **Rule.** When a phase changes the setup, capture the K84 byte constants again from a dry run
  of the new setup, and compare them with the old constants before the tests use them.
- **Why.** The constants pin the setup state at the time of capture. A later setup step that
  changes the same accounts makes the old constants wrong, and a test that uses them then checks a
  state that does not exist.
- **Input needed.** A dry run log with the setup byte lines.
- **Status.** Seen on 1 target.

### K59. A rare multi-step check path gets a deterministic setup smoke, and its canary fires through the smoke

- **Rule.** When a check needs a path of 3 or more steps that random runs reach rarely, add a
  setup smoke that runs the path and calls the check, and make the canary of the check fire in a
  dry run through that smoke.
- **Why.** A random canary run on a rare path can give 0 buckets when the canary works, so a miss
  proves nothing. A smoke gives the check a judged line in every run and a certain canary result
  in about 1 minute.
- **Input needed.** A setup that can reach the path without random draws.
- **Status.** Seen on 1 target.

### K60. A supplementary build needs its own dry run in each phase

- **Rule.** When a phase keeps a supplementary build (a flag that changes the setup), do a dry
  run of that build once in each phase, before its campaign run.
- **Why.** Setup changes of later phases are tested only with the normal build. A supplementary
  build can fail in setup for several phases, and nobody sees it until its run.
- **Input needed.** The list of supplementary build flags of the harness.
- **Status.** Seen on 1 target.

---

## 3. Actions and inputs

### K12. Every actor can call every instruction

- **Rule.** Give each action a signer index over all actors (admin, authorities, users). Do not
  hard-code the correct caller.
- **Why.** Access-control bugs show only when a non-privileged actor sends a privileged
  instruction.
- **Input needed.** Actor list from `setup()`.
- **Status.** Seen on 2 targets.

### K13. Model state changes that bypass the program

- **Rule.** For each program-controlled token account, add a "donate" action (direct SPL Token
  transfer). Add the same for lamports into program accounts. Add direct transfers of receipt
  tokens (LP or shares) between users.
- **Why.** Anyone can send tokens or lamports to an account without the program. A harness that
  models only program instructions misses these states.
- **Input needed.** Token accounts and lamport accounts whose authority is a program PDA; receipt
  token mints.
- **Status.** Seen on 2 targets.

### K14. Bound amounts with `#[range]`, then clamp

- **Rule.** Put `#[range(0..=max_meaningful)]` on each amount parameter (max = start balance or
  supply). Then clamp to the balance. Return early when the call is certain to fail. For slippage
  arguments, use `None` or the expected amount plus or minus a small fuzzed delta.
- **Why.** `.min(balance)` on a raw `u64` gives "all of the balance" in almost every call. The
  balance drains and later calls send 0. A random slippage value makes the slippage error
  dominate.
- **Input needed.** Start balances from `setup()`; slippage arguments in the IDL.
- **Status.** Seen on 2 targets. The slippage part is a design decision: the dominance of a
  random slippage value is a hypothesis, not tested by a run.

### K15. In a batched transaction, put the relayer first

- **Rule.** In each `send_batch`, put the relayer as the first entry of `.signers()` in the first
  `add_transaction`. Do not rely on `.fee_payer()`.
- **Why.** `send_batch` uses the first unique signer as the fee payer and ignores `.fee_payer()`
  (pitfall C2). A wrong fee payer breaks K38 without an error.
- **Input needed.** Crucible `send_batch` source.
- **Status.** Seen on 1 target.

### K61. Make a destructive environment state rare in the selector

- **Rule.** When one selector value of an environment action makes every price-loading action
  fail (for example a stale clock), give that value a small share (1 in 8 or less), not 1 in 2.
- **Why.** With a 50 % stale clock, a large part of each sequence fails on the price check. The
  fuzzer then spends executions on the error path and the value invariants see fewer real flows.
- **Input needed.** Which environment states block which actions.
- **Status.** Seen on 1 target. Confirmed by a review and a blocker run; no A/B run.

### K62. A loss path needs a curve that is filler-favorable at the time the generator fills

- **Rule.** If the program trades along a time-based price curve, find where the curve crosses
  the fair price before you set the scales. Set the scales so that the curve is filler-favorable
  at the fill time of the generator.
- **Why.** If the curve is below fair at the fill time, no fill can lose value for the program.
  The loss branch is then dead, and the reach line stays 0.
- **Input needed.** The curve formula, the window length and the fill time distribution.
- **Status.** Seen on 1 target.

### K63. A multi-op action that stops at its first failure changes state while `success` is false

- **Rule.** For an action that sends 2 or more operations and stops at the first failure, record
  each operation result. Checks that read `success` (fee frame, write set, "no change on
  failure") must treat a partial run as a change.
- **Why.** The first operations commit before the failing one. A check that sees
  `success == false` expects the state of before, and it fires on correct program behavior.
- **Input needed.** The operation list of the action.
- **Status.** Seen on 1 target.

### K64. A harness price solver must bound its result; a dust leg gives an extreme price

- **Rule.** When a harness action solves an oracle price for a target ratio (for example a loan-to-value ratio),
  refuse a result above a fixed factor of the current price (K14 return). Test the bound with the
  case of a dust leg.
- **Why.** The solved price scales with 1 / (leg amount). A dust leg gives a price that no real
  oracle reports. The external program can then panic on overflow, and every later prelude fails.
  Checks then fire on a state that no attacker can make (oracle prices are trusted).
- **Input needed.** The current oracle price at the action.
- **Status.** Seen on 1 target.

---

## 4. Account choice and shadows

### K16. "Honest accounts" harnesses hide account-validation bugs

- **Rule.** Cover 2 attack surfaces: values and accounts. Cover accounts with
  `--mutate-accounts` (K36) and with action parameters that select each account slot from a pool,
  independently of the signer (K20, K21).
- **Why.** On mainnet an attacker chooses every account of an instruction, also keys of other
  users. A harness that derives all accounts from the actor index lets the fuzzer change only
  values and order.
- **Input needed.** For each instruction account: signer flag, `UncheckedAccount` or weak
  constraints (`#[derive(Accounts)]`).
- **Status.** Seen on 1 target.

### K17. Missing signer: substitute harvested privileged keys

- **Rule.** Build a key dictionary from all pubkeys stored in program state plus all actors. For
  each non-signer slot, let the fuzzer pick a key from it, while only an unprivileged attacker
  signs. Use an invariant on the privileged state as the oracle (ghost state updated by signer,
  see `invariant-catalog.md`, class 5). Cheap variant: use a relayer as fee payer and put the
  authority in `.signers()` (K38).
- **Why.** A missing-signer bug passes the identity check. Only the exact stored key passes, so
  random substitution does not find it.
- **Input needed.** Pubkey fields of state accounts (IDL `accounts` types); IDL `signer` flags.
- **Status.** Seen on 1 target.

### K18. Account-binding bugs need consistent multi-account substitutions

- **Rule.** For each key stored in program state (an "anchor" reference), generate a substitution
  that replaces the referenced account and every account locally bound to it, as one consistent
  set. Shadow objects (K19) supply these sets.
- **Why.** A single substitution fails a local constraint (for example `token::mint`) and hides
  the missing binding to stored state.
- **Input needed.** Pubkey fields in state accounts; local constraints (`token::mint`, `has_one`,
  `seeds`).
- **Status.** Seen on 1 target.

### K19. Create realistic shadow objects

- **Rule.** For each honest mint that matters, create a shadow mint (same decimals, attacker
  authority) and a token account of it for each actor. For each external-program account, add a
  shadow with data that the external program can produce. For each initialize instruction
  without access control, add an attacker-created instance. Add attacker-created token accounts
  with any owner (for example owned by a pool PDA).
- **Why.** Random accounts fail relation checks. Unrealistic shadows give false positives.
- **Input needed.** Mints; initialize instructions and their signer constraints; external account
  layouts.
- **Status.** Seen on 2 targets.

### K20. Typed account registry: what to keep and what to avoid

- **Rule.** Register every account with a kind and an origin. Give each slot a typed pool from
  its declared type. Keep the registry append-only, so indices stay stable. Do not deduplicate
  inputs by account combination. Bucket crashes at report time (K32). Use the soft bias of K21 for
  the honest account, not a fixed "pool index 0 = honest" rule.
- **Why.** Combination dedup ignores state (the same combination can fail before a deposit and
  succeed after it). It also breaks determinism of replay and `tmin`. A registry that is not
  append-only changes the meaning of saved inputs.
- **Input needed.** Slot types from the IDL and source.
- **Status.** Seen on 1 target. Implemented as a setup-only registry (no SVM scan). Open: an SVM
  scan for accounts that actions create; keys in state that are not in the registry.

### K21. Soft bias toward the honest account

- **Rule.** Map each fuzzed slot value so that most values (75%) pick the honest account and the
  rest cover every other type-valid candidate.
- **Why.** Near-uniform choice makes most calls fail at the first constraint. A hard cap on
  deviations removes combinations; a soft bias removes none.
- **Input needed.** Honest account per slot; pool size per slot type.
- **Status.** Seen on 1 target. 75% is the only tested bias value (1 comparison against
  near-uniform choice). Use it as a start value, not as an optimum.

### K22. Count the registry before you choose the slot range

- **Rule.** Count the entries of each filter pool from the setup plan. Choose `SLOT_RANGE` so that
  `SLOT_RANGE - HONEST_CUTOFF` is at least the largest pool. Keep `HONEST_CUTOFF / SLOT_RANGE`
  at 75%.
- **Why.** With a too small range, some candidates are unreachable. A range of 256 (64 deviation
  values) fits a small target only.
- **Input needed.** Setup account list per kind.
- **Status.** Seen on 2 targets.

### K23. Use stateless mode for account-choice fuzzing

- **Rule.** Use stateless mode for multi-slot account attacks. Use `--stateful` when a bug needs
  a long valid prefix. Running both is cheap.
- **Why.** In stateless mode an input with 1 correct deviation reaches a new error path, stays in
  the corpus and is mutated toward the second deviation. In stateful mode a failed action makes
  no new state.
- **Input needed.** None.
- **Status.** Seen on 1 target. Hypothesis, not tested by a run: the mechanism has no controlled
  test.

### K24. Gate attack actions after their bug is confirmed

- **Rule.** After an attack-capable action finds its bug, gate or remove it. Then explore the
  honest paths again from realistic states.
- **Why.** Attack successes move the state far from normal (drained vault, extreme share price).
- **Input needed.** Failure histogram per run.
- **Status.** Seen on 1 target.

---

## 5. Invariants

### K25. Conservation over a closed token set is harness sanity, not a bug detector

- **Rule.** Mark closed-set conservation checks as "harness sanity". For theft, write per-actor
  accounting invariants (value in against value out).
- **Why.** SPL Token moves tokens only between passed accounts. If the harness passes only tracked
  accounts, the sum can change only by a burn or a harness bug. It does not see a redistribution
  between users.
- **Input needed.** Token accounts that the harness passes; mint and burn authorities.
- **Status.** Seen on 1 target.

### K26. A global price invariant is blind to redistribution

- **Rule.** Pair a global price-per-share invariant with a per-actor wealth invariant. Bound the
  gain of the signer and the loss of an honest actor.
- **Why.** A loss of one holder while the price goes up passes the price check. Transitions with
  supply 0 are skipped.
- **Input needed.** Per-actor ghost ledger or valuation of all actor holdings.
- **Status.** Seen on 2 targets. The loss bound for a non-signer actor is a hypothesis, not
  tested by a run.

### K27. Compare sibling instructions for asymmetric checks

- **Rule.** For paired instructions (deposit and redeem, lock and unlock, borrow and repay,
  mint and burn), compare their account constraints and `require!` checks. A check that is in one
  and not in the other is a lead and a good target for an invariant or a shadow.
- **Why.** Bugs often hide in the less used side of a pair.
- **Input needed.** `#[derive(Accounts)]` structs; handler `require!` calls.
- **Status.** Seen on 1 target.

### K28. Zero crashes needs a liveness check of the oracle

- **Rule.** For each invariant, confirm once that it can fire: a canary (temporary violating
  action or patched check), or a design case that must trigger it (K29).
- **Why.** A wrong account, a wrong offset or an always-true condition also gives 0 crashes.
- **Input needed.** One violating action per invariant.
- **Status.** Seen on 2 targets. On those targets the canary step was skipped, so the liveness
  of the invariants that never fired is not proven. K29 is the practical replacement.

### K29. Check that each expected bucket appears

- **Rule.** List the buckets that must appear (design cases from the spec). After the first short
  runs, check that each one appears. If one does not, print the computed values for the
  triggering action before you trust any result.
- **Why.** A harness defect can silence an invariant. A missing expected bucket exposed the
  first-wins defect of K33.
- **Input needed.** List of expected buckets.
- **Status.** Seen on 1 target.

### K65. A failure-side agreement check needs a skip for "no transaction sent" and a model charge

- **Rule.** A check that compares the code of a failed transaction with the model code must skip
  the case where the action sent no transaction (`requested == None`, the K14 early return). The
  model must also apply the fee charge that the failed transaction would have made. A failed
  transaction reverts this charge, so the pre-state of the chain is the state without the charge.
- **Why.** Without the skip, an action that stopped before the send gives a false bucket. Without
  the model charge, the model reads a share price that the chain never had, and the model and the
  chain give different codes.
- **Input needed.** The list of early returns of each action; the order of the fee charge and the
  guards in the program.
- **Status.** Seen on 1 target.

### K66. A canary must change a rule that fires often, not a boundary equality

- **Rule.** Build a canary by a change to a rule that fires on many draws (for example drop a cap
  step so that every capped failure gives a model success). Do not build it from a boundary
  equality (for example `>` instead of `>=`). A boundary equality occurs in a few draws, so a 60 s
  canary run can miss it.
- **Why.** A canary proves that the check can fire. A canary that fires only on an exact boundary
  does not prove it in a short run and costs extra runs.
- **Input needed.** For each changed check, the rate of the draws that reach the changed rule.
- **Status.** Seen on 1 target.

### K67. Check a property that combines 2 model results for independence from the existing chain checks

- **Rule.** If a property combines the results of 2 models (for example a deposit model and a
  withdraw model), check that it is independent of the checks that already compare each model with the
  chain. If the property follows from the model definitions, it cannot fail while those checks
  pass. Use values from the chain only, or do not count the property as program coverage.
- **Why.** Such a property passes by arithmetic. It adds run lines but no detection power, so its
  reach count gives false confidence.
- **Input needed.** The list of the checks that compare each model with the chain.
- **Status.** Seen on 1 target.

### K68. A wealth check that names the signer needs a rule for a permissionless action whose fee moves value to the signer's shares

- **Rule.** If a check names the signer and the action is permissionless, write a rule for the
  case where the action fee mints shares to the signer. The skip must name the hidden case in the
  skip catalog.
- **Why.** A permissionless settler that holds shares gains NAV from the fee of another owner. A
  wealth invariant then reports a gain that is not a defect.
- **Input needed.** The fee path of each permissionless action.
- **Status.** Seen on 1 target.

### K69. A strict code clause must follow the full guard order of the handler, also guards that another check owns

- **Rule.** Before a check demands one error code, list every guard that the handler runs before
  the code site, in program order. Skip the clause when any earlier guard can fail. Reuse the
  predicate of the check that owns the earlier guard (for example the predicate of a price-bound
  check), so that the 2 checks cannot disagree.
- **Why.** A handler returns the code of the first failing guard. A spec row names the code of 1
  guard only. Each guard that the spec does not list gives a false bucket when the generator
  makes 2 guards fail at the same time.
- **Input needed.** The handler source between the account load and the code site.
- **Status.** Seen on 2 targets.

### K70. A check that reads an action fact that only some actions set skips always on the other actions

- **Rule.** Before a check uses an action fact (a field of the last-action record), list the
  actions that set it. If the checked action does not set it, compute the fact from the state at
  the check, not from the action record.
- **Why.** An unset fact has its default value. The check then takes the same skip on every call
  of that action, and the judged count stays 0 with no error.
- **Input needed.** The writers of each last-action field (grep the field name in the action
  modules).
- **Status.** Seen on 1 target.

### K71. A check with an empty write mask fails on the first config call of a field that the program settles on update

- **Rule.** Before a phase adds a config field to a draw, list the fields that the program changes
  as a side effect of the update (a window settle, a timestamp). Give the write-set check the
  expected values of those fields, from the existing model.
- **Why.** A write-set check with an empty mask for a config row sees the side effect as a byte
  outside the write set. The bucket is false, and it appears only when the field enters the draw
  for the first time.
- **Input needed.** The list of setter side effects in the program state module for each new
  config field.
- **Status.** Seen on 1 target.

---

## 6. Crash bucketing and triage

### K30. Read the failure histogram after every short run

- **Rule.** After each short run, group failures by (action, error code). Flag each group that is
  larger than the successful calls of that action.
- **Why.** The histogram shows a blocker at once.
- **Input needed.** Harness `[DEBUG]` output.
- **Status.** Seen on 2 targets.

### K31. Decode an Anchor error code with its instruction

- **Rule.** Decode an error code together with the instruction that raised it. Better: parse the
  `Error Code: <Name>` line of the program logs. Anchor 2000-2999 = constraints, 3000+ = account
  errors, 6000+ = program errors.
- **Why.** Each `#[error_code]` enum starts at 6000, so 2 modules can use the same code.
- **Input needed.** Error enums per module; transaction logs.
- **Status.** Seen on 1 target.

### K32. Invariant crashes are not deduplicated: bucket them

- **Rule.** Report invariant violations through a bucketing function, not plain `fuzz_assert!`,
  when an action can break an invariant in many ways. Bucket key = (invariant ID, action, relation
  of each non-honest slot). Keep every run log, because the violation message is only in the log.
  Group buckets by (invariant, action) at triage time. Use bucketing and K24, not
  `--stop-on-crash` or a disabled action.
- **Why.** Invariant crash files are named by input hash. Each new input that breaks the
  invariant is a new file.
- **Input needed.** Run log; `*.meta.json`.
- **Status.** Seen on 1 target.

### K33. Crucible keeps only the first violation of an iteration

- **Rule.** Consume a crash bucket only when Crucible records its message. Track "a violation
  was recorded" for the whole iteration in a fixture field. Do not reset it inside the invariant
  function; the per-iteration fixture clone resets it.
- **Why.** `record_violation` is first-wins per iteration (pitfall C1). A bucket that is marked as
  seen while its message was dropped is lost for the whole run.
- **Input needed.** Crucible violation API semantics.
- **Status.** Seen on 2 targets.

### K34. One bucket per cause: split invariant IDs by known cause

- **Rule.** When a doc contradiction or a design case can trigger an invariant, give that case its
  own ID (for example an ID for 1 action type, or for a pre-state condition). Treat buckets that
  differ only in slot relations as possible duplicates in triage.
- **Why.** A known cause consumes the general bucket and hides later violations with other causes.
- **Input needed.** Doc contradictions from the spec stage; pre-state fields that select the case.
- **Status.** Seen on 1 target.

### K35. Mutation crash files are overwritten across runs

- **Rule.** Give each run its own `--crashes-out <dir>`. Run `crucible tmin` before the next run.
  Read "same crash id" as "same (class, instruction)", not "same input".
- **Why.** The mutation crash file name comes from the finding ID
  `{class}:{program}:{discriminator}` (pitfall P2). A later run that finds it again writes the
  same file.
- **Input needed.** Crash metadata `mutation_finding_id`.
- **Status.** Seen on 1 target.

### K72. Probe CC-13 on a signer wallet that is a token authority is the known P4 pattern

- **Rule.** When `--mutate-accounts` reports `CC-13 forwarded-account` for a signer wallet that
  the program forwards to SPL Token or Token-2022 as transfer or burn authority, classify it as a
  false positive (pitfall P4 row 2) without a replay.
- **Why.** The token program checks only the signature of the authority. The owner and the data
  of the wallet account do not change the result.
- **Input needed.** Setup registry map (address to role).
- **Status.** Seen on 2 targets.

### K73. Compute Anchor error codes with a parser that accepts an explicit discriminant

- **Rule.** When an Anchor error enum starts with an explicit value (`FirstError = 1000`),
  compute each code as `6000 + start + position` with a parser that counts the `= N` line too.
  Assert the codes that the harness uses in a unit test.
- **Why.** A line filter for `Name,` skips the first variant `Name = 1000,`. Every code is then
  1 too low, and histogram checks and liveness filters test the wrong failures.
- **Input needed.** The error enum source.
- **Status.** Seen on 1 target.

### K74. `crucible show --replay` uses the release binary that exists; build it after a harness fix

- **Rule.** After a change to the harness, run `cargo build --release --features invariant_test`
  in the harness folder before a crash replay. Check the binary time against the source time.
- **Why.** `crucible show` does not build. `cargo check` and `cargo test --lib` do not write
  `target/release/invariant_test`. A replay on the old binary reproduces the old bucket and looks
  like a failed fix.
- **Input needed.** None.
- **Status.** Seen on 1 target.

---

## 7. Probe engine (`--mutate-accounts`)

### K36. Always run a probe pass after coverage saturates

- **Rule.** Run `--mutate-accounts` in its own crash directory after the blocker loop. It needs a
  harness where each instruction succeeds sometimes, with token accounts and their mints in the
  instruction.
- **Why.** The probes need no oracle and no domain knowledge. A probe pass can find a missing
  `has_one` with no harness change.
- **Input needed.** Harness with a high success rate for each instruction.
- **Status.** Seen on 2 targets.

### K37. Score the two oracle types separately

- **Rule.** Keep 2 oracle types apart in scoring: differential probes (missing account checks;
  need only reachability) and invariants (logic and economic bugs; need domain knowledge). A
  harness must supply both: high success rates for probes, property invariants for logic.
- **Why.** They find different bug classes.
- **Input needed.** Crash metadata field `Mutation finding: yes/no`.
- **Status.** Seen on 1 target.

### K38. Use a relayer as fee payer and put every authority in `.signers()`

- **Rule.** Use one dedicated relayer as fee payer for every action. Pass every actor that acts
  as an authority in `.signers()`, also when the IDL does not mark the account as signer. "Acts as
  an authority" = the key is compared with a stored key, or the account name is an authority name
  (owner, admin, authority, user, depositor). Do not register the relayer as a slot candidate.
- **Why.** CC-4 skips the fee payer (pitfall P3). A separate payer makes the authority a
  `SuppliedSigner` candidate. A relayer in the registry can be substituted into fee recipient
  slots and distort fee checks.
- **Input needed.** `require!(x.key() == state.field)` patterns; IDL account names.
- **Status.** Seen on 2 targets.

### K39. Know the limits of the probes

- **Rule.** Use probes as the default account-check pass. Add fuzzed account choice (K20, K21) for
  weakly constrained slots when multi-step or effect-changing exploits matter. Spend invariant
  effort first on economic properties.
- **Why.** Probes test only coded strategy shapes. Their oracle requires the same effects as the
  baseline. They test 1 transaction in isolation, once per (instruction, path).
- **Input needed.** Weakly constrained slots; privileged follow-up actions.
- **Status.** Seen on 1 target.

### K40. Candidate probe: payer-as-authority

- **Rule.** (Idea for a Crucible extension.)
  1. Select as candidate a fee payer key that is also in a non-signer meta.
  2. Keep the candidate only if a program-owned account stores the key.
  3. Send the probe with a new funded fee payer. The key signs nothing.
  4. Report a finding if the probe has the same effects as the baseline.
- **Why.** CC-4 skips the fee payer. The Crucible docs call payer-as-authority cases
  "intentionally conservative" (`docs/constraint-check-engine.md`).
- **Input needed.** Strategy API in `crucible-test-context/src/account_mutation.rs`.
- **Status.** Seen on 1 target. Hypothesis, not tested by a run: idea only, not implemented.

### K41. Exclude the harness's helper instructions from the probe pass

- **Rule.** Add `.skip_account_mutation()` to every raw helper call (System or Token transfers
  that model user behavior). Probe only target-program instructions.
- **Why.** A system transfer to any account succeeds, so CC-3 fires. Each distinct amount gives a
  new finding ID, because the discriminator prefix of a System transfer contains amount bytes
  (pitfalls P2, P4, P5).
- **Input needed.** List of non-target instructions that the harness sends.
- **Status.** Seen on 1 target. The fix is not applied in a run yet.

---

## 8. Coverage

### K42. Edge growth finds blockers; it does not measure bug-finding power

- **Rule.** Use edge growth only to find harness blockers. Measure bug-finding power by invariant
  quality and by executions to the first finding.
- **Why.** Small programs and stateless corpora saturate fast.
- **Input needed.** `FUZZ_PULSE` edges over time.
- **Status.** Seen on 2 targets.

### K43. `FUZZ_PULSE` is event-driven

- **Rule.** For final statistics, use the `Timeout reached` line or `FUZZ_STATS_CSV=<path>`. Do not
  use the last pulse.
- **Why.** A pulse line appears when the corpus grows. The last pulse can be long before the end.
- **Input needed.** Run log or stats CSV.
- **Status.** Seen on 1 target. `FUZZ_STATS_CSV` is a hypothesis, not tested by a run.

### K44. Read coverage percentages with their denominator

- **Rule.** Compare the LCOV file list with the program source file list. Report "files never
  reached" separately. Use coverage to find unreached handlers and branches, not as a bug metric.
- **Why.** Files with 0 hits after setup are not in the LCOV file, so they are not in the
  denominator. Setup code is not measured. A high line percentage can coexist with critical bugs.
- **Input needed.** Program source file list; LCOV `SF:` records.
- **Status.** Seen on 2 targets.

### K45. Map each coverage gap to a cause

- **Rule.** Map each uncovered block to (a) an action that does not exist, (b) a tier that is not
  built, or (c) a rare state. Add actions for (a) when the block is a security check.
- **Why.** An unexercised check is an untested check.
- **Input needed.** LCOV per-line data; source of each uncovered block.
- **Status.** Seen on 1 target.

### K75. Crucible keeps only source files under the folder before `/target/` in the `--symbols` path

- **Rule.** Put the DWARF symbols file in a path without `/target/` (for example
  `.cov/symbols.so`) when the DWARF build is in another folder than the repo. Then Crucible keeps
  every absolute path that is not a registry or toolchain path.
- **Why.** `is_user_source` in `crucible-test-context` 0.3.0-alpha.1 (`src/coverage/lcov.rs`)
  takes the text before `/target/` in `FUZZ_SYMBOLS` as the workspace root and drops other
  absolute paths. A DWARF build in a scratch copy has scratch paths, so every program file is
  dropped and the log shows `Source-level coverage: 0 source files`.
- **Input needed.** The DWARF path prefix (`llvm-objdump -d -l symbols.so | grep -m1 handler_`).
- **Status.** Seen on 1 target.

### K76. Crucible attaches the `--symbols` DWARF map to every loaded program

- **Rule.** In a harness that loads more than 1 SBF program, keep only the LCOV block of the
  target program. Find it by its size (most files) and its content (the program handlers). Drop
  the other source-level blocks.
- **Why.** The generated `init_dwarf_maps` (`crucible-fuzz-macro` 0.3.0-alpha.1, `codegen.rs`)
  inserts the same map for every program in `get_program_coverage_totals`. So the PCs of the
  external program fixtures map to false lines of the target program. The log shows 1
  `Source-level coverage: N source files` line for each program.
- **Input needed.** The program count in the setup.
- **Status.** Seen on 1 target.

### K77. A coverage replay of a narrow focus corpus and the main corpus must be read as a union

- **Rule.** When a phase has a focus run, compute the coverage union of the focus and the main
  corpus before it reports lost lines. Report the main corpus loss separately as a corpus effect.
- **Why.** The main corpus of a `--max-actions 4` run loses multi-step paths between phases (K97).
  Alone, the loss looks like a regression of the harness.
- **Input needed.** The 2 LCOV files and the target program block of each.
- **Status.** Seen on 1 target.

### K78. A focus remap of the action draw removes the paths it maps away

- **Rule.** When a focus build maps action selectors to the focus action, list the lost actions in
  the plan, and read the focus coverage only as a union with the main corpus.
- **Why.** The remap makes the focus action more frequent, but the focus corpus then has no input
  for the mapped actions, and their handler files leave the replay.
- **Input needed.** The selector table of the remapped action.
- **Status.** Seen on 1 target.

### K79. A union of branch outcomes of 2 LCOV files needs the same side order

- **Rule.** Before a union of `BRDA` outcomes of 2 coverage replays, check that each shared line
  has the same side order in both files. Report the union of outcomes as a range when the order
  differs, or report the union of lines only.
- **Why.** The order of the outcome sides of a line can differ between 2 replays of the same
  binary (for example on derive-macro lines). A merge by position then counts complementary sides
  as hit, and the union looks higher than it is.
- **Input needed.** The 2 LCOV files and the target program block of each.
- **Status.** Seen on 1 target.

---

## 9. Process and review

### K46. Partition a large target by external dependency

- **Rule.** Before the harness design, list each instruction with its programs, sysvars, owner
  constraints and remaining accounts. Group instructions into tiers by the most expensive
  dependency. Build tier by tier; each tier has its own spec, plan and runs.
- **Why.** One harness for all instructions puts every setup blocker into one long loop.
- **Input needed.** `Accounts` structs (`Program<...>`, `address = ...`, `owner`), CPI calls,
  remaining-account comments, state dependency chain.
- **Status.** Seen on 1 target.

### K47. Review the spec and the plan with an independent model before coding

- **Rule.** Have an independent model review the written design against the program source and
  the fuzzer source before coding. Repeat for the plan, and for the code before the first long
  run. Give the reviewer the blind rules.
- **Why.** Each defect would show in a dry run only as an unclear failure.
- **Input needed.** Spec, plan, program source, fuzzer source.
- **Status.** Seen on 1 target.

### K48. Read the diff of a fix against the root cause

- **Rule.** For the fix of a semantic bug, read the diff against the root-cause statement. Do not
  rely on the implementer's report only.
- **Why.** A report can describe a wrong change as part of the fix.
- **Input needed.** Root-cause statement; fix diff.
- **Status.** Seen on 1 target.

### K80. A tier can need a complex external program in setup only

- **Rule.** When you partition by external dependency, check the setup path separately from the
  actions. If the initialize instruction makes a CPI into a complex program, load that program in
  `setup()` only and keep it out of every action of the tier. State this in the tier plan.
- **Why.** The playbook tier rule ("tier 1: SPL Token only") reads as if tier 1 cannot load a
  complex program at all. Without this rule, the vault cannot exist in tier 1.
- **Input needed.** CPIs of each initialize instruction; which state each action needs.
- **Status.** Seen on 1 target. Hypothesis, not tested by a run.

### K81. Use lens-based property discovery as a gap check after the doc list, and expect leads more than run buckets

- **Rule.** After the doc-derived property list and a formula facts file exist, do a single discovery
  pass with 5 property lenses and a synthesizer. Give the lenses the list of existing invariants
  and the harness limits. Take a lean core of the synthesis (about 12 properties) into the spec,
  not all of it. For each lens property that needs a new input mode, prove in a unit test at the
  post-setup state that the mode reaches the property before the spec is approved (see K84).
  Record the lens leads at once.
- **Why.** The doc list gives bounds and rules ("not lowered", "at most"). The lenses give exact
  check forms with rounding, frame conditions and multi-action properties that the doc list does
  not have. The lens pass also reads the code closely, so it finds audit leads before any run.
  Lens properties that need a new input mode are the expensive part: the lenses do not check
  that the mode can succeed at the setup state.
- **Input needed.** The doc property list, a formula facts file with file:line, the list of
  existing invariants, the harness limits (action count, tolerance, snapshot model).
- **Status.** Seen on 1 target.

### K82. Put the phase forms of the existing checks before the new actions

- **Rule.** Add the phase forms of the existing checks first. Then add the new actions. Each
  action task then ends with a 0-bucket gate.
- **Why.** A bucket from a new action is then a defect of the action, not a missing form of a
  check.
- **Input needed.** The list of the existing checks that the new actions touch.
- **Status.** Seen on 1 target.

### K83. Generate the plan patches from a scratch build history

- **Rule.** Build the plan from a scratch build history (a clone where each task is applied in
  order). Cut each patch from that history, so that every hunk applies.
- **Why.** Patches written by hand drift from the real file state, and a hunk fails during the
  work.
- **Input needed.** The base commit and the task order.
- **Status.** Seen on 1 target.

---

## 10. Harness models and unit tests

### K84. Compute near-zero input values at the post-setup state, with the full fee and the minimum on the right amount

- **Rule.** Before a spec states that a boundary input value succeeds, compute it with the exact
  model at the state after setup: the share supply and total assets after the setup deposits, the full
  fee (bps part and fixed minimum), and each minimum on the amount that the program checks
  (gross or net). Put the values from the dry-run log into a unit test that asserts success and
  the failure code of the neighbor values.
- **Why.** A value computed from the seed share price, or with only part of the fee, or with the
  minimum on the wrong amount, fails on every draw. The property that needs the success side then
  never runs, and a 15 s gate shows only a 0 reach count, not the cause.
- **Input needed.** The dry-run setup values (share supply, total assets, token states); the program guard
  order and which amount (gross or net) each minimum uses.
- **Status.** Seen on 1 target.

### K85. A randomized model test needs small supplies and fractional pending values in its domain

- **Rule.** A randomized test of a model formula must draw small share supplies, small amounts and
  pending values with a fractional part. The test base must use the same fractional part as the
  program (for example `B = S0 * q + (pend0 mod q)`).
- **Why.** With large whole numbers the rounding steps of the program have no effect, and a wrong
  formula passes. With a whole-number base, a correct formula fails when `pend0` has a fraction.
- **Input needed.** The scale of the values in the program (share supply, quantum, pending fees).
- **Status.** Seen on 1 target.

### K86. Check a "fails with code X" claim of a summary at the setup fees

- **Rule.** Before the spec states that an input fails with a given code, compute it with the
  exact model at the setup fees and the setup minimums. Do not copy a claim from a summary without
  this computation.
- **Why.** A claim that holds at one fee setting can be false at another. This is the failure side
  of lesson K84.
- **Input needed.** The setup fees, the minimums and the guard order of the program.
- **Status.** Seen on 1 target.

### K87. A test that proves a model panics on a short vector needs an enabled entry past its end

- **Rule.** In a test that proves a model rejects or panics on a vector shorter than the state,
  enable an entry at an index past the end of the vector. A state with only disabled entries past
  the end does not test the length.
- **Why.** A guard such as `enabled && deficit[j] > 0` skips the index of a disabled entry, so the
  short vector is never read past its end. The test passes for the wrong reason.
- **Input needed.** The guard of each loop that indexes the vector.
- **Status.** Seen on 1 target.

### K88. A harness model of an external deposit must include the rounding of the external program

- **Rule.** When a check compares an expected token outflow with a limit, compute the outflow with
  the external program's rounding (port it from the external program source; a deposit that
  converts through an exchange rate can move less than the requested amount), on the external
  state that the instruction reads.
- **Why.** The external program can move less than the requested amount. A model that uses the
  requested amount gives a 1-unit false positive at the limit.
- **Input needed.** A port of the external rounding into the harness model.
- **Status.** Seen on 1 target.

---

## 11. Reach lines and gate runs

### K89. Count the successes of a multi-step action with a stderr line, and judge reach on the main run

- **Rule.** When an action can succeed only after another action sets a precondition in the same
  sequence (for example a flag, then an admin call), print 1 stderr line per success with the
  key values. Check the count in the blocker loop and again in the main run. Do not block the main
  run on 0 successes in 60 s blocker runs; plan a supplementary run if the main run also has 0.
- **Why.** `discovered: N/N actions` counts calls, not successes. A two-step precondition is rare
  in short runs and appears when the corpus grows. Without the line, the invariants on that path
  look live but never run.
- **Input needed.** Preconditions of each action (flags, roles, prior state).
- **Status.** Seen on 1 target.

### K90. A gate-on state that only a rare config draw sets needs a setup flag and an extra run

- **Rule.** If a rare config draw sets the state that a check needs (for example a swap mode),
  add a setup flag that sets the state at the start. Run the flag in an extra run.
- **Why.** The main run reaches the state about once. The success side of the check has no
  evidence otherwise.
- **Input needed.** The config draw that sets the state, and its rate.
- **Status.** Seen on 1 target.

### K91. A reach line that counts attempts proves nothing about success

- **Rule.** Write a gate rule with success lines only. A skip line or an attempt line does not
  satisfy a gate.
- **Why.** An attempt count above 0 hides a success count of 0.
- **Input needed.** For each gate, the success line that the rule uses.
- **Status.** Seen on 1 target.

### K92. A failure-boundary reach line counts fresh draws only, so it needs a focus flag run

- **Rule.** If a reach line needs an input at the boundary of a failure (for example a request
  just over a limit), add a focus flag that draws the boundary on purpose. Run the flag in an
  extra run, as for K90.
- **Why.** A failed action adds no corpus entry. The corpus cannot keep and mutate a boundary
  input, so only fresh random draws reach the line.
- **Input needed.** The failure code and the input range that gives the boundary.
- **Status.** Seen on 1 target.

### K93. A stub action never counts as discovered

- **Rule.** Do not count a K14 stub action in the discovered-actions pass rule. Set the rule to the
  number of real actions.
- **Why.** A stub has no effect, so a discovery of the stub proves nothing. A rule that counts it
  hides a real action that the fuzzer never reaches.
- **Input needed.** The list of stub actions and the phase that fills each one.
- **Status.** Seen on 1 target.

### K94. A pass-rule line that needs an object of an earlier action is a 2-step line

- **Rule.** Mark a pass-rule line as a 2-step line if it needs an object that an earlier action
  creates (for example an order). Do this also when its own action has no state precondition.
- **Why.** The count of such a line depends on the input distribution. A short run can show 0 for a
  harness that works. The main run judges the line (K89).
- **Input needed.** For each line, the chain of earlier actions that it needs.
- **Status.** Seen on 1 target.

### K95. Reach and measure names with lower-case letters need `[A-Za-z]` in every grep

- **Rule.** Use the class `[A-Za-z0-9-]` in each grep that counts reach lines, measure lines or
  skip lines. Do not use `[A-Z0-9-]`.
- **Why.** A name such as `CPI-deposit` has lower-case letters. A grep with upper-case letters only
  drops these lines, and the count shows 0 or a short list.
- **Input needed.** The name pattern of each reach and measure line.
- **Status.** Seen on 1 target.

### K96. A gate that compares the line counts of 2 runs needs a noise model

- **Rule.** Do not stop a gate on the ratio of a line count between 2 short runs. Stop only on a
  presence change: a skip key that no base run has, or a line at 0 that is above 0 in every base
  run. Record the ratios only. A presence rule does not see a check that stops early without a
  skip line, so run the old canaries again.
- **Why.** A 60 s run follows its own corpus path. 2 runs of 1 harness differ by up to 2 orders of
  magnitude in a line count, so a ratio rule stops the gate when nothing changed.
- **Input needed.** At least 2 base logs of the old harness and the list of allowed skip keys.
- **Status.** Seen on 1 target.

### K97. New actions dilute a multi-step flow; gate the flow lines of earlier phases, not only the new lines

- **Rule.** When a phase adds actions, include the 2-step lines of the earlier flows (for example
  the fill lines) in the gate run and compare them with the last main run. If they fall, plan a
  stateful run or a focus mode for them before the main run.
- **Why.** Each new action takes a share of the action draws, and long actions lower the execution
  rate. A flow of 2 or more steps falls with the product of its step shares. Short gate runs do
  not show the fall.
- **Input needed.** The line counts of the last main run (compare script).
- **Status.** Seen on 1 target.

### K98. A 2-step reach line needs a gate run of 120 s or more, not 15 s or 60 s

- **Rule.** For a line that needs 2 actions in order (a state change, then the action), set the
  gate run to 120 s or more, with 1 repeat when the line is 0. For a 3-step line, use the stateful
  run (K102).
- **Why.** The rate of a 2-step line is low and varies much between runs. A short gate gives 0 on
  correct code, and the 0 looks like a broken path. A higher action limit in stateless runs does
  not fix it.
- **Input needed.** The number of actions in order that the line needs.
- **Status.** Seen on 1 target.

### K99. A measure line before the send shows which step of a multi-step chain is rare

- **Rule.** When a check needs a chain of 2 or more actions in order, print 1 measure line for each
  step attempt before the send (with the target and the key flags), and pair it with the outcome
  line of the same action in a script.
- **Why.** A reach line counts only the end of the chain. When it is 0, it does not show which step
  fails, and a longer run then only doubles the time.
- **Input needed.** The outcome lines of each step, and a block rule that ties an outcome to its
  measure line.
- **Status.** Seen on 1 target.

### K100. A deterministic smoke that prints reach lines puts them in every run log; count the run part only

- **Rule.** When a setup smoke prints the same reach, step or measure lines as the action, count
  those lines in the run part of the log only (the lines after the last `[SETUP]` line). Give the
  command with the pass rule, for example
  `awk 'FNR==NR{if(/^\[SETUP\]/)n=FNR;next} FNR>n' <log> <log>`.
- **Why.** The smoke runs in every setup, so each log has its lines before the first iteration. A
  count over the whole log can pass a rule that the random run did not meet (a gate with 0 random
  reach shows 1 or 2 smoke lines).
- **Input needed.** A smoke that prints reach lines, and a log with the `[SETUP]` marker lines.
- **Status.** Seen on 1 target.

### K101. A focus chain step whose post state adds little field novelty is seldom saved in the stateful pool; route more modes to the aim

- **Rule.** When a focus chain has a step that changes 1 small field (for example 1 `u64` in a
  bucket that other states already have), do not rely on the pool to save its post state. Route
  more action modes to the aim, so that the next step of the chain follows in the same action
  chain.
- **Why.** A post state enters the stateful pool only with field novelty or new edge coverage. A
  small change with no prelude gives neither. The next draw then starts from another state, and
  the chain restarts. With a draw rate of about 1/90 for each step, a chain of 3 steps gives about
  1 % per chain.
- **Input needed.** The step list of the chain and the pool rules of the Crucible version.
- **Status.** Seen on 1 target.

---

## 12. Stateful and long runs

### K102. Judge lines of 3 or more steps on a stateful run, and buckets on the stateless run

- **Rule.** Run a stateful run next to the stateless main run when the pass rule has lines of 3 or
  more steps. Judge those lines on the stateful run. Judge the buckets and the 1-step lines on the
  stateless run.
- **Why.** A stateful iteration continues from a pool state that keeps the earlier steps of a
  chain, so a long chain does not multiply the step rates. A pool state holds no violation, so the
  stateful bucket list is a lower bound. A pool state also moves away from the setup state, so
  1-step lines of the setup state are lower.
- **Input needed.** The chain length of each pass-rule line (K94).
- **Status.** Seen on 1 target.

### K103. Size the stateful pool to the machine memory before parallel stateful runs

- **Rule.** Before 2 or more stateful runs at the same time, set `--pool-size` so that runs x
  workers x pool size x state size fits in the RAM. `--pool-size` is a limit per worker, so the
  total pool of a `-j N` run is about N times the value. A short test does not fill the pool;
  measure the RSS at a full pool, or compute it from the bytes per state.
- **Why.** The default pool (256,000 states) of a large target can use more memory than the
  machine has. Parallel stateful runs with the default pool can stop the machine.
- **Input needed.** Machine RAM; worker count (`-j`); bytes per pool state.
- **Status.** Seen on 1 target.

### K104. Do not seed a stateful run with `--corpus-in`

- **Rule.** Start stateful runs with an empty corpus. Use the stateful corpus only for coverage
  replay.
- **Why.** A stateful run seeded with an earlier stateful corpus gave false buckets over many
  checks. The same binary without the seed corpus gave none.
- **Input needed.** None.
- **Status.** Seen on 1 target.
