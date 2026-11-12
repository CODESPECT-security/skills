# Reusable code patterns

This folder has 1 file to copy and 8 patterns to rewrite for a new target. The excerpts use a
generic vault program with a share token. They are short on purpose. Do not
copy a whole `main.rs`; it is target-specific.

Validated with crates.io `crucible-*` 0.3.0-alpha.1, LiteSVM 0.15.2, `anchor-lang` 1.0.1.

ID legend: `K1`-`K104` are lessons in `../lessons.md` (search for `### K19.`). Letter-number IDs
(`B6`, `C1`, `R3`, `T4`, ...) are pitfalls in `../crucible-pitfalls.md`. Section numbers such as
`3.8` and `5.2` are in `../invariant-catalog.md`.

| Pattern | Source file | Lessons |
|---------|-------------|---------|
| 1. Account registry and bucketing | `account_fuzzing.rs` (copy) | K19-K22, K32-K34 |
| 2. Harness layout: library + binary | `lib.rs`, `Cargo.toml` | |
| 3. Doc formulas in program integer order | `model.rs` | K84-K88, `invariant-catalog.md` 5.2 |
| 4. Synthetic external accounts | `external_data.rs` | K19 |
| 5. Chain helpers and ghost snapshots | `chain.rs` | K6 |
| 6. Environment model in setup | `main.rs` setup | K5, K7 |
| 7. FeeCheck pre-state | `main.rs` deposit/withdraw actions + invariant | `invariant-catalog.md` 3.8 |
| 8. Flash-loan batch with relayer first | `main.rs` `action_flash_loan` | K15, K38 |
| 9. Environment actions | `main.rs` `action_env_*` | `invariant-catalog.md` 4.4 |

## 1. Account registry and crash bucketing (`account_fuzzing.rs`)

Copy `account_fuzzing.rs` into `src/`. Its header lists the parts that you adapt. Use it in this
order:

1. In `setup()`, register every account with a kind and an origin. Use `add_actor` for wallets that
   may sign. Do not register the relayer (K38).
2. Call `add_shadow_mints` for the mints that need shadows.
3. Call `reg.print_map()` and `reg.print_pool_sizes()` at the end of `setup()`. No `[WARN]` line may
   appear.
4. In each action, pick each slot through the registry and store the tag:

```rust
let mut tag = ActionTag::new("deposit");
let r = &self.reg;
let vault = r.pick(&mut tag, "vault", Filter::Pda("Vault"), Some(h.vault), s_vault);
let user_shares = r.pick(&mut tag, "user_share_account", Filter::TokenAccount, Some(h.user_share_account), s_user_shares);
self.last_tag = tag;
self.last_signer = Some(me);
```

Slot parameters use the same range as `SLOT_RANGE`: `#[range(0..512)] s_vault: usize`. The honest
value `h` comes from a helper that builds the honest accounts struct (`deposit_accounts(p, user)`).

5. In the invariant, collect violations, then report them all at the end:

```rust
let mut found: Vec<(&'static str, String)> = Vec::new();
// ... checks push ("ACCESS", format!(...)) ...
for (id, msg) in found {
    report_violation(id, &fixture.last_tag, &mut fixture.tainted, &mut fixture.violation_recorded, msg);
}
```

Fixture fields:

```rust
tainted: Vec<&'static str>,
/// A violation was recorded in this iteration (Crucible keeps only the first).
/// Reset only by the per-iteration fixture clone; never reset inside invariant_test.
violation_recorded: bool,
```

**Caution.** Do not reset `violation_recorded` in the invariant function. The invariant runs after
every action, so a reset there re-creates the first-wins defect (K33, K48).

Known limit: `tainted` hides a second bug that breaks the same invariant later in the same
iteration.

## 2. Harness layout: library + binary

Put pure code (formulas, encoders) in a library target with no Crucible dependency. Then
`cargo test --lib` runs fast and checks the formulas before any fuzz run.

```toml
# Cargo.toml of the harness (added to the crucible init template)
[lib]
name = "target_fuzz"
path = "src/lib.rs"
```

```rust
// src/lib.rs
pub mod model;
pub mod external_data;
```

The binary (`main.rs`, `chain.rs`, `account_fuzzing.rs`) uses `target_fuzz::model`.
Compile check: `cargo check --features invariant_test` (`crucible-pitfalls.md`, B4).

## 3. Doc formulas in program integer order (`model.rs`)

This is the port model of `invariant-catalog.md` 5.2. Write a docs model (rounding in favor of the
protocol) next to it for the bounds.

Rules:

1. Write each doc formula as a function, in the same integer order as the program (multiply first,
   then divide, same divisors). Then expected values match to the lamport.
2. Use `u128` and `saturating_mul` for products. Return 0 for a zero denominator.
3. Write one unit test for each worked example in the docs. Add edge tests: zero supply, large
   values (about 1e15 × 1e15), epoch boundaries.
4. Keep tolerance constants here, with a comment on what each one covers.

```rust
pub const FEE_DIVISOR: u128 = 10_000;

/// Port of a fee formula from the target docs, in the program integer order.
pub fn fee_amount(amount: u64, fee_bps: u64) -> u128 {
    (amount as u128) * (fee_bps as u128) / FEE_DIVISOR
}

/// NAV per share did not go down: (nav_new + tol) / shares_new >= nav_old / shares_old.
/// Undefined (true) when either supply is 0.
pub fn nav_per_share_ok(nav_old: u128, shares_old: u64, nav_new: u128, shares_new: u64, tol: u128) -> bool {
    if shares_old == 0 || shares_new == 0 {
        return true;
    }
    (nav_new + tol).saturating_mul(shares_old as u128) >= nav_old.saturating_mul(shares_new as u128)
}

#[test]
fn fee_matches_docs_example() {
    // Docs example: a fee of 30 bps on 1,000,000 units is 3,000 units.
    assert_eq!(fee_amount(1_000_000, 30), 3_000);
}
```

Write 1 such test for each worked example in the target docs.

## 4. Synthetic external accounts (`external_data.rs`)

Use this when the program only **reads** an account of another program (no CPI). Then the harness
writes the account data and does not load the other program (K46).

Rules:

1. Copy the struct layout from the external program source of the exact version in the target
   `Cargo.lock`. Count the fields against that source (a count from memory or from docs can be wrong by 1 field).
2. Encode by hand with the serialization rules (borsh: 1-byte enum and `Option` tags, little-endian
   integers). This avoids crate conflicts between the external crate and the harness `anchor-lang`.
3. Define offset constants for every field that the harness reads or changes.
4. Test: total length, each field at its offset, and that a write changes only its field.
5. Create the account with the external program as owner:

```rust
ctx.create_account().pubkey(ext).owner(EXTERNAL_PROGRAM).owner_unverified().data(&data).create().unwrap();
```

```rust
pub const VALUE_OFFSET: usize = 72; // from the layout, not from this example
pub const ENCODED_LEN: usize = 200;

pub fn write_value(data: &mut [u8], v: u64) {
    data[VALUE_OFFSET..VALUE_OFFSET + 8].copy_from_slice(&v.to_le_bytes());
}

#[test]
fn write_value_changes_only_that_field() {
    let mut d = sample().encode();
    let before = d.clone();
    write_value(&mut d, 2_000);
    assert_eq!(read_value(&d), 2_000);
    assert_eq!(&d[..VALUE_OFFSET], &before[..VALUE_OFFSET]);
    assert_eq!(&d[VALUE_OFFSET + 8..], &before[VALUE_OFFSET + 8..]);
}
```

Realism rule: a shadow of an external account holds only data that the external program can
produce (K19). Example: an attacker-created external account cannot use a mint whose authority is the honest
instance.

## 5. Chain helpers and ghost snapshots (`chain.rs`)

Keep read helpers, clock control and ghost snapshot types in one module.

Clock control (`crucible-pitfalls.md`, R3). In setup, first set the schedule:
`ctx.set_sysvar(&EpochSchedule::custom(SLOTS_PER_EPOCH, SLOTS_PER_EPOCH, false));`

```rust
/// `warp_to_slot` changes only `clock.slot`, so the epoch is written as well.
pub fn set_slot(ctx: &mut TestContext, slot: u64) {
    ctx.warp_to_slot(slot);
    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.slot = slot;
    clock.epoch = slot / SLOTS_PER_EPOCH;
    clock.leader_schedule_epoch = clock.epoch + 1;
    ctx.set_sysvar(&clock);
}
```

Read helpers: `lamports(ctx, &key)` (0 if missing), `mint_supply` (bytes `[36..44]`),
`read_vault` (`ctx.read_anchor_account::<Vault>(..).ok()`), `read_ext` (data with a length check).
Return `Option` or a default, so an invariant never panics on a substituted account.

Ghost snapshots: a `Config` struct with the fields that a config instruction writes, and an
`Immutable` struct with the fields that never change. Both derive `Clone, Debug, PartialEq` and have
`fn of(&Vault) -> Self`. The invariant compares `Config::of(&vault)` with `fixture.expected_config[i]`.
Ghost update by signer:

```rust
fn refresh_config_ghost(&mut self, vault: Pubkey, signer: Pubkey) {
    if let Some(i) = self.vault_index(&vault) {
        if self.vaults[i].authority == signer {
            if let Some(v) = read_vault(&self.ctx, &vault) {
                self.expected_config[i] = Config::of(&v);
            }
        }
    }
}
```

## 6. Environment model in setup

Load built-in program versions and protect them from the coverage override (K5, K7):

```rust
let cov_override = std::env::var("FUZZ_PROGRAM_SO").ok();
std::env::remove_var("FUZZ_PROGRAM_SO");
ctx.add_program(&TOKEN_PROGRAM, "programs/spl_token-3.5.0.so").unwrap();
if let Some(v) = cov_override {
    std::env::set_var("FUZZ_PROGRAM_SO", v);
}
```

Loud setup: every setup transaction goes through a helper that prints the logs and panics on
failure. Print the epoch schedule, the clock, the registry map and the pool sizes.

```rust
fn expect_success(step: &str, outcome: anyhow::Result<TxOutcome>) {
    match outcome {
        Ok(TxOutcome::Success { .. }) => eprintln!("[SETUP] {step}: OK"),
        Ok(TxOutcome::ProgramError { error, logs, .. }) => {
            eprintln!("[SETUP] {step}: FAILED {error:?}");
            for log in &logs { eprintln!("    {log}"); }
            panic!("setup failed at {step}");
        }
        Err(e) => panic!("setup send error at {step}: {e}"),
    }
}
```

Assert frozen parameters after setup (example: `assert_eq!(vault.fee_bps, 30)`).

## 7. FeeCheck pre-state

The action computes the expected window before the send. It records the balances of the receiving
accounts. After a successful send, it stores a check for the invariant.

```rust
#[derive(Clone)]
enum FeeCheck {
    Withdraw { value: u128, min_fee: u128, max_fee: u128, receiver: Pubkey, receiver_before: u64,
               treasury: Pubkey, treasury_before: u64, vault: Pubkey },
    Deposit { asset_cost: u128, cap: u128, user: Pubkey, user_before: u64, design_case: bool },
}
// fixture field: pending_fee: Option<FeeCheck>
```

In the invariant:

```rust
match fixture.pending_fee.take() {
    Some(FeeCheck::Deposit { asset_cost, cap, user, user_before, design_case }) => {
        let paid = user_before.saturating_sub(lamports(&fixture.ctx, &user)) as u128;
        let total_fee = paid.saturating_sub(asset_cost);
        if total_fee > cap + 1 {
            let id: &'static str = if design_case { "FEE_CAP_CASE" } else { "FEE_CAP" };
            found.push((id, format!("{id} deposit fee {total_fee} > cap {cap} + 1")));
        }
    }
    // Withdraw: skip when two roles are the same account (receiver == treasury; treasury == vault).
    _ => {}
}
```

Rules: use `take()`, so a check runs once. Store the pre-state flag that selects a design-case ID
(`design_case`). Do not store a check when two roles are the same account; record the skip.

## 8. Flash-loan batch with the relayer first

`send_batch` uses the first unique signer as fee payer (`crucible-pitfalls.md`, C2). Put the
relayer first in the first queued instruction only:

```rust
let borrow = |ctx: &mut TestContext, amt: u64, first: bool| {
    let b = ctx.program(pid).call(instruction::FlashStart { amount: amt }).accounts(/* .. */);
    if first { b.signers(&[&*relayer, &*signer]) } else { b.signers(&[&*signer]) }.add_transaction()
};
// queue: borrow(first = true), optional middle instruction, repay(first = false)
let outcome = match self.ctx.send_batch() {
    Ok(Some(o)) => Ok(o),
    Ok(None) => Err(anyhow::anyhow!("empty batch")),
    Err(e) => Err(e),
};
```

Variants: 0 = borrow + repay; 1 = borrow + middle + repay; 2 = borrow only; 3 = repay only. The
middle instruction kind is a fuzzed index. In the reference harness, the middle instruction used
honest accounts of an independently fuzzed vault, to limit the number of parameters. Name each
variant as its own action tag (`flash_mid_deposit`), and set the design-case kind (deposit) for the
middle deposit. After success, update the ghost state for a middle config instruction.

## 9. Environment actions

An environment action changes the world, not the program. Mark it, so economic checks skip it:

```rust
pub fn action_env_value_change(&mut self, #[range(0..2)] ext_idx: usize, #[range(0..251)] delta_raw: u64) -> bool {
    self.last_tag = ActionTag::new("env_value_change");
    self.last_kind = LastKind::Env;
    self.last_signer = None;
    // ... read total, new_total = total + total * (delta_raw - 50) / 10_000 (-0.5% .. +2%)
    self.ctx.update_account(&ext, |data| external_data::write_value(data, new_total)).is_ok()
}

pub fn action_env_advance_slots(&mut self, #[range(0..100_001)] slots: u64) -> bool {
    self.last_tag = ActionTag::new("env_advance_slots");
    self.last_kind = LastKind::Env;
    self.last_signer = None;
    let target = self.ctx.slot() + slots;
    set_slot(&mut self.ctx, target);
    true
}
```

In the invariant, skip the wealth and NAV checks when `last_kind == LastKind::Env`, but refresh
all baselines (`invariant-catalog.md`, 3.9, 4.4).
