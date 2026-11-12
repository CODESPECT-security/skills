---
name: crucible-fuzzing
description: Use when fuzzing a Solana or Anchor program with Crucible (the `crucible` CLI, `crucible-test-context`, `crucible-idl-gen`, LiteSVM), or when you plan, design, build, run or triage a Crucible harness, or read its crashes, `--mutate-accounts` probe results, stateful runs or coverage.
---

# Crucible fuzzing campaigns

## Overview

A Crucible campaign finds bugs only when 2 conditions are true: the harness is generic (every
actor, every account slot, every signer can vary), and each check is proven live (it can fire,
and the path to it is reached). Most campaign failures come from Crucible and LiteSVM facts that
are not obvious. This skill gives the gates and the facts. The full procedure and the evidence are
in `references/`.

## Procedure

Follow `references/playbook.md` (20 steps, stages A-F). Each stage ends with its output and gate:

| Stage | Output | Gate before the next stage |
|-------|--------|----------------------------|
| A. Frame | Protocol: open or blind. Who runs which commands | The user chose both |
| B. Understand | Intake note (playbook step 3). Tier table if more than about 12 instructions (K46) | Build works; IDL format known (K49); runtime token program known (R4) |
| C. Design | Spec with every slot of the template below, within the size limit | User approved the review step; review applied (K47); user approved spec v2 |
| D. Build | Harness; unit tests of each model formula | `cargo test --lib` passes; dry run passes with the role map and pool sizes |
| E. Run | Blocker loop, canaries, main run, probe pass | Pass rule of each run met (template slot 8) |
| F. Evaluate | Triage of every bucket; coverage; notes; new lessons | Each bucket classified; each gap mapped to a cause (K45) |

Read `references/crucible-pitfalls.md` sections B and R before the first build. Read section S
before the first stateful run, and section V before coverage.

## Required design spec slots

Write the spec with these 10 slots, in this order. Each slot is required. Write "none, because ..."
when a slot does not apply.

**Size.** The spec has at most 1,500 words for a program with up to 10 instructions, plus 150 words
for each further instruction (count with `wc -w`). A slot is a table or a list: 1 row or 1 line per
fact, with a file:line or a K/pitfall ID as its source. Before you finish, count the words. If the
count is above the limit, move derivations and code to an appendix file and keep the result in
the spec.

1. **Protocol and leads.** Open or blind (playbook step 1). Bugs that you suspect from reading go to
   a lead list (or the sealed note, if blind). Each lead gets a canary check. The lead does not
   change the generic harness design (slot 4).
2. **Versions and runtime.** Anchor version and IDL path (K49, B3). Rust toolchain of the harness
   (Crucible needs 1.89 or later). Token program that LiteSVM loads (R4, K5). Fixture programs:
   strip `.symtab` (B6); run each external instruction once (K54).
3. **Fee payer and signers.** 1 funded relayer pays every transaction. Each authority goes into
   `.signers()`. Never make the actor the fee payer: CC-4 skips the fee payer (K38, P3, C2).
4. **Account choice.** A typed registry of every account, with shadow mints and a token account
   per actor (K18, K19, K20). Each action picks each account slot from the registry with a soft
   bias to the honest account (K21, K22). Harvested privileged keys for signer substitution (K17).
   Do not write 1 action per suspected bug instead of this.
5. **Actions.** 1 action per instruction. Every actor can call every instruction (K12). 1 action
   per state change outside the program (K13). Amount ranges with clamp and early return (K14).
   Raw helper calls use `.skip_account_mutation()` (K41, P5).
6. **Checks.** Table: ID, property, class (`references/invariant-catalog.md`), exact form, what it
   hides. Report each through `report_violation` with its ID (K32). Only the first violation of an
   iteration is kept (C1, K33). Conservation over a closed set is harness sanity only (K25).
   These 3 lines are required:
   - **2 models per formula.** A port model (program order) for agreement and codes, and a docs
     model (rounding in favor of the protocol) for bounds. Each difference between them is a lead
     (catalog 5.2).
   - **Split flow.** For each fee formula, price formula, minimum and time-based multiplier: n
     small operations do not beat 1 large one (catalog 3.11).
   - **Fair valuation.** Wealth and NAV checks value every asset that the program holds at a fair
     price, not with the program's own NAV formula (catalog 3.10, section 4).
   - **Rounding table.** List every division, shift, cast to a smaller type and explicit rounding
     call on a value path (fees, prices, shares, multipliers, conversions; search the source for
     `/`, `checked_div`, `>>`, `as u64`, `floor`, `ceil`). One row each: file:line, expression,
     direction, who gains (user or protocol), and the check that sees it. Each row where the user
     gains is a lead and needs a check that can fire (a docs model bound or a split flow), not
     only an action that reaches it. Put the table in the appendix (catalog 5.2).
   - **Same-kind accounts.** List every account slot that the program checks only by type, mint
     or authority (no fixed address, no seeds, no `has_one`). For each, the registry holds 2 or
     more valid accounts of that kind, and a check fires when value or state goes to a
     non-canonical one (for example: the sum over all program-owned accounts of a mint differs
     from the amount in the canonical account, or the program's recorded amount). A path alone
     is not enough (catalog 3.3).
7. **Liveness.** For each check: a canary that changes a rule that fires often (K66), and a success
   reach line (K91). For each action: a success line, not an attempt line. A list of the buckets
   that must appear (K29). For chains of 2 or more steps: K98, K102, K59.
8. **Run plan.** Table: run, command, duration, pass rule, who runs it. Order: dry run, blocker loop
   with failure histogram (K30), canary runs, main run (stateless), stateful run, probe pass
   (K36), coverage replay. Stateful runs start with an empty corpus (K104, S2). Size
   `--pool-size` per worker to the RAM (K103, S1).
9. **Triage.** Bucket by (check ID, action) from the log, because crash files have no message and
   no dedup (T2, K32, K34). Rebuild the release binary before each replay (K74, T8). Put crash
   directories under `<fuzz_dir>/crashes/` for `tmin` (T4). Check probe crashes against P4.
   Decode error codes per instruction and per enum start (K31, K73).
10. **Review and risks.** Known risks and blind spots. The review focus: 3 to 6 questions for the
    reviewer, in priority order.

## Independent review (separate step)

The independent review of the spec (K47) is a separate step after the spec is written. Do not
start it as part of the design task. When the spec is complete:

1. Tell the user that the spec is ready for review, with the review focus of slot 10.
2. Give the expected cost: 1 reviewer subagent, about 10 minutes and about 2 to 3 USD for a small
   program.
3. Start the review only after the user approves it.
4. Check each blocker in the source yourself. Write spec v2 with a change table.

The plan review (playbook step 7) follows the same rule.

## Crucible facts that agents get wrong

| Fact | Ref |
|------|-----|
| CC-4 (`--mutate-accounts` signer probe) skips the fee payer. An authority that pays the fee is never probed | P3, K38 |
| `send_batch` uses the first unique signer as fee payer | C2 |
| LiteSVM 0.15 loads p-token as the SPL Token program; it rejects `SyncNative` with more than 1 account | R4 |
| Only the first violation per iteration is recorded | C1 |
| Invariant crash files are named by input hash; the message is only in the run log. Keep every log | T2 |
| `--corpus-in` on a stateful run gives false buckets | S2 |
| `--pool-size` is a limit per worker; the default pool can need tens of GB | S1 |
| `crucible show --replay` does not build; it uses the old release binary | T8 |
| `tmin` reads only `<fuzz_dir>/crashes/<test>/` and overwrites the file | T4 |
| Slots do not advance per transaction; `warp_to_slot` does not move the epoch | R2, R3 |
| A panic in an action or in a check kills the fuzzer process | C6 |
| `--max-depth` default differs in the docs (15, 100); set it | T5 |
| A 2-step line needs runs of 120 s or more; a 15 s gate shows 0 on correct code | K98 |

## Common mistakes

| Mistake | Fix |
|---------|-----|
| The harness tests only the bugs that you found by reading | Slot 1 for leads, slot 4 for generic account choice |
| A check never fires and you report "0 crashes" | Canary and reach line for each check (slot 7) |
| A bucket after a harness change is called a program bug | Replay on a rebuilt binary; read the guard order of the handler (K69) |
| An exact model check fails on correct code | Port the program rounding; compute boundary values at the post-setup state (K84, K85, K88) |
| Run durations chosen by feeling | Each run has a pass rule (slot 8) |

## References

| File | Use it when |
|------|-------------|
| `references/playbook.md` | You start or continue a campaign (20 steps) |
| `references/lessons.md` | You make a design decision (K1-K104, by topic; index at the top) |
| `references/crucible-pitfalls.md` | A build, setup, run, probe, replay or coverage step fails |
| `references/invariant-catalog.md` | You choose and write checks; section 5 for exact models |
| `references/templates/` | You write the registry, account choice and crash bucketing code |

The lessons come from 3 audit campaigns on Anchor programs of different size. The **Status** line
of each lesson gives the number of targets that showed it.
