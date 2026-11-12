# Invariant catalog

This file lists invariant classes for a Crucible harness. For each class, it gives the method to
compute the check, its tolerances and its false-positive traps. Lesson IDs `Kn` point to
`lessons.md`. Code patterns are in `templates/README.md`.

## 1. Overview

| # | Class |
|---|-------|
| 1 | Conservation over a closed set |
| 2 | Supply consistency |
| 3 | Accounting against real balance |
| 4 | State machine |
| 5 | Access control, ghost state by signer |
| 6 | Immutability |
| 7 | Monotonic counter |
| 8 | Fee bounds from docs |
| 9 | Per-actor wealth |
| 10 | NAV (assets) per share |
| 11 | Split flow |

## 2. Common rules

1. Take each property from the docs or from a generic class. In a blind campaign, put your own
   code suspicions in a sealed note, not in the first invariants (`playbook.md`, step 13).
2. Run every invariant after every action (`crucible-pitfalls.md`, C4). Keep baselines in the
   fixture; the invariant function gets `&mut fixture`.
3. Report through `report_violation` with an invariant ID, not plain `fuzz_assert!`, when an action
   can break the invariant in many ways (K32, K33).
4. Give each design case and each doc contradiction its own ID (K34). The main ID then stays free
   for other causes.
5. Compute expected values with the program's integer order. Put the formulas in a library with
   unit tests from the doc examples (`templates/README.md`, "model.rs").
6. Record each tolerance and what it hides in the design notes. Do not widen a bound without the
   user's approval.
7. Skip a check only for a stated reason (overlapping accounts, undefined ratio, environment
   action). Record each skip as a blind spot.
8. List the buckets that must appear and check them after the first runs (K29).

## 3. Classes

### 3.1 Conservation over a closed set

- **Property.** The sum of token balances over a fixed set of accounts is constant.
- **When to use.** As harness sanity, in the first harness version.
- **How to compute.** Sum `token_balance` over all tracked accounts; compare with the start total.
- **Tolerance.** None.
- **False-positive traps.** None known.
- **Limits.** SPL Token moves tokens only between passed accounts. If the harness passes only
  tracked accounts, the sum changes only by a burn or a harness bug. It does not see theft between
  users (K25).

### 3.2 Supply consistency

- **Property.** A mint supply equals a stored counter, or the sum of all holder balances.
- **When to use.** For each receipt token (shares) that the program mints and burns.
- **How to compute.** Mint supply = bytes `[36..44]` of the SPL Mint account (no helper in
  `TestContext`). Compare with the counter (for example `total_shares`) or with the sum of
  tracked holders.
- **Tolerance.** None.
- **False-positive traps.** "Sum of holders" fails if an untracked account can hold the token
  (for example a shadow account or a transfer to an untracked owner). The counter form fails if a
  holder burns receipt tokens directly through SPL Token (a burn needs only the holder's
  signature). Add a direct-burn action (K13) and decide: supply <= counter, or exclude burned
  amounts with a ghost ledger. This trap is not tested yet.

### 3.3 Accounting against real balance

- **Property.** The real balance of a vault covers the value that the program records for it.
- **When to use.** For each program account that holds lamports or tokens and has a stored
  accounting field.
- **How to compute.** Example for a vault that holds lamports in a system account:
  `lamports(vault_account) >= vault.recorded_lamports + rent(0 bytes)`.
- **Tolerance.** None. Use `>=`: donations (K13) make the real balance larger.
- **False-positive traps.** Rent: include the rent-exempt minimum of the vault account.
- **Same-kind accounts.** If the program accepts any account of a kind for a slot (checked only
  by mint and authority), put 2 or more such accounts in the registry. Check that the canonical
  account holds the recorded amount, and that no other program-owned account of that kind
  receives value. A value split across accounts of 1 kind breaks the accounting of later
  instructions that read only the canonical one.

### 3.4 State machine

- **Property.** A transient state is closed at the end of each transaction.
- **When to use.** Flash loans, locks, "in progress" flags.
- **How to compute.** Example: a stored loan amount is 0 after each action. The
  invariant runs between transactions, so a correct program always shows the closed state.
- **Tolerance.** None.
- **Negative variants.** Add actions that open without close and close without open (flash
  variants 2 and 3). Their failures are expected and show in the failure histogram.

### 3.5 Access control with ghost state updated by signer

- **Property.** A privileged field changes only in a transaction that the stored authority signed.
- **When to use.** Owner, admin, authority, config fields.
- **How to compute.**
  1. Keep a ghost copy of the privileged fields in the fixture (for example `expected_owner`, or
     `expected_config[vault]`, a `Config` snapshot of all config fields).
  2. Before the call, read the stored authority.
  3. After a successful call, update the ghost only if the **signer** was that authority. Do not
     update it by the account slot (K17).
  4. The invariant compares the chain with the ghost.
- **Which signer.** The fuzzed signer index, never the fee payer. The fee payer is the relayer
  (K38).
- **Tolerance.** None.
- **Traps.** Batched transactions: update the ghost for the inner instruction too (for example, a
  config instruction in the middle of a flash batch calls `refresh_config_ghost`). Exclude
  counters from the config snapshot.

### 3.6 Immutability

- **Property.** Fields that never change after initialization stay the same.
- **When to use.** Always. It is cheap.
- **How to compute.** Whole form: the account bytes equal the bytes after setup. Field form: an
  `Immutable` snapshot (for example `authority`, `vault`, `share_mint`, `bump`).
  Use the field form when the account also has counters.
- **Tolerance.** None.

### 3.7 Monotonic counters

- **Property.** A cumulative counter never decreases.
- **When to use.** Accrued fees, total minted, epoch counters.
- **How to compute.** Keep `last_accrued[vault]`; check `new >= last`; update the baseline in the
  invariant.
- **Tolerance.** None.

### 3.8 Fee bounds from the docs, in lamports

- **Property.** The fee of one operation is inside the window that the docs define.
- **When to use.** Every fee formula in the docs.
- **How to compute.** The FeeCheck pre-state pattern (`templates/README.md`):
  1. Before the send, compute the expected value and the fee window from the pre-state, with the
     program's integer formulas, in the program's order of multiply, divide and round.
  2. Record the balances of the receiving accounts before the send.
  3. After a successful send, store a `FeeCheck` in the fixture.
  4. The invariant takes the `FeeCheck` (`Option::take`), computes the real fee from the balance
     changes and checks the window.
- **Compare in lamports, not in percent.** Compute the window ends with the same integer formulas.
  This prevents false positives on very small trades.
- **Tolerances.** Each recipient part plus or minus 1 lamport when the program splits a fee
  (hides rounding of the split). Fee cap
  plus 1 lamport (hides rounding of the cap).
- **Design-case IDs.** When you find a doc contradiction in a fee rule, give its case a separate
  ID. This keeps the main fee ID free for other causes (K34).
- **False-positive traps.**
  - Overlapping accounts: the fee recipient slot equals the user's receive account, the payer, or
    the vault. The observed balance changes are then wrong. Skip that check and record the skip
    (one skip condition for each pair of overlapping roles).
  - A relayer in the registry can be substituted into a fee recipient slot and receive or pay
    transaction fees (K38).
  - A fuzzed config instruction can set the fee recipient to a vault account.

### 3.9 Per-actor wealth

- **Property.** One action by actor *u* does not increase the wealth of *u* beyond the documented
  bound.
- **When to use.** Every economic protocol. It sees redistribution that a global price check
  misses (K26).
- **How to compute.**
  1. Wealth of *u* = native balance + Σ (each asset held × fair price) + Σ over vaults
     (share balance × NAV per share).
  2. Keep `last_wealth[actor]` for all actors. Refresh it in every invariant call.
  3. Check only the signer of the last action (`last_signer`). The fee payer is the relayer, so the
     signer pays no transaction fees.
  4. Skip the check after an environment action (prices change); still refresh the baseline.
- **Tolerances and what they hide.**
  - `ROUNDING_TOL` = 1 lamport per integer division in the sum (example: 4 = 3 vaults + 1 NAV
    floor). It hides floor errors smaller than the tolerance.
  - Documented bound of the action (for example a fee-based bound that the docs state for 1
    action type), stored before the send. It hides gains up to that bound.
- **Design-case IDs.** When 1 action type has a known reason to break the check (a design case or
  a doc contradiction), give it its own ID. The main wealth ID then stays free for other actions.
- **False-positive traps.**
  - Rent included in NAV gives a gain on the first honest deposit. Use the accounting value plus
    valued inventory, not the raw lamports.
  - Assets valued with an attacker-set price (a shadow of an external price account) make the check blind for value
    that moves through that asset.
  - Donations and direct receipt transfers: the sender loses, which is not a gain; the receiver
    is not the signer, so it is not checked.

### 3.10 NAV (assets) per share

- **Property.** The value of one receipt token does not decrease from a non-environment action.
- **When to use.** Every vault or pool with a receipt token.
- **How to compute.**
  - Single asset: `vault_balance / share_supply` never goes down. Compare without division:
    `new_bal * old_supply >= old_bal * new_supply` in `u128`.
  - With inventory: NAV = accounting value + Σ (inventory × fair price). Check
    `(nav_new + tol) * shares_old >= nav_old * shares_new` (`model::nav_per_share_ok`).
  - Skip when the old or new supply is 0 (undefined; first deposit or all redeemed).
  - Keep `last_nav[vault]` for all vaults. Refresh in every invariant call.
- **Tolerances.** Same as 3.9.
- **Design-case IDs.** Same rule as 3.9.
- **Valuation.** Compute NAV from the assets that the program holds, each at a fair price (section
  4), not with the program's own NAV or share-price formula. A check that calls the program
  formula cannot see an asset that the formula leaves out.
- **Limits.** Blind to a loss of one holder while the price goes up (K26). Effects at supply 0 are
  not checked.

### 3.11 Split flow

- **Property.** A flow split into n smaller operations does not give the actor a better result
  than the same flow in 1 operation: the total fee paid is not lower, and the total output is not
  higher, beyond the rounding bound of n operations.
- **When to use.** Every fee formula, price formula, minimum amount and time-based multiplier
  (for example a multiplier that grows with epoch progress).
- **How to compute.** A split action: 1 actor sends the same flow as n legs (n from a small set,
  for example 2, 10, 100) in 1 sequence. Compute the 1-operation result from the pre-state with
  the docs model (5.2). Compare the sum of the n legs with it.
- **Tolerance.** n times the rounding bound of 1 operation. Record what it hides. A leg below a
  documented minimum must fail; if it succeeds, that is the finding.
- **Traps.** State changes between legs (price, epoch, fee accrual) change the 1-operation value.
  Freeze the environment during the split action.

## 4. Multi-asset vaults

This section applies the methods of classes 3.3, 3.8, 3.9 and 3.10 to a vault that holds N
assets. It states no fact about a specific target.

### 4.1 Inventory of every asset

1. List every asset that the vault can hold: deposit tokens, tokens received by swaps or trades,
   reward tokens, lamports. Include assets that arrive without the program (donations, K13).
2. For each asset, list every account that holds it for the vault (token accounts owned by a vault
   PDA, lamport PDAs).
3. NAV = Σ over assets (amount held × price). For token assets, read the token account balance
   from the chain. For lamport accounts, subtract the rent-exempt minimum, or use the program's
   accounting value plus valued inventory (trap in 3.9). Record which choice you made.
4. Also compute the program's own accounting value. If it differs from NAV by design, give the
   deposit and withdraw cases their own IDs (K34).

Reason: a program can price receipt tokens with an accounting value that excludes an asset that
the vault holds (for example tokens that the vault received in a trade).

### 4.2 One price source per asset

1. Use one fair price source for each asset. Compute it in the harness from the state that the
   program reads (external pool data, oracle account), with the doc formula in program integer
   order.
2. Put each price formula in the `model.rs` library with unit tests from the doc examples.
3. If a price source is an account that the harness writes (synthetic oracle or external pool
   data), write it with a byte-exact encoder with offset tests (`templates/README.md`, "Synthetic
   external accounts").
4. Freeze parameters that would give one asset 2 prices (example: one rate parameter has the same
   fixed value in all vaults, checked in setup).

### 4.3 Valuation of assets outside the accounting

- Value every asset that the vault holds, also assets that the program accounting does not record
  (inventory, donations). Record which assets the program counts and which it does not.
- An asset whose price an attacker controls (attacker-created external pool or mint) cannot be
  valued with a fair price. Record it as a blind spot of 3.9 and 3.10.

### 4.4 Price changes as environment actions

1. Add one environment action per price source that moves the price inside a realistic range
   (example `action_env_value_change`: -0.5% to +2%, negative = loss).
2. Add one environment action that moves the clock (example `action_env_advance_slots`: 0 to 1/4
   epoch). Write the full `Clock` (`crucible-pitfalls.md`, R3).
3. If the program checks price staleness, a clock action without a price update creates stale
   prices. This tests the staleness checks. This step is not tested yet. Do not assume one oracle provider: the
   target can read any oracle program, or several. For each oracle account that the target reads,
   copy its layout from the source of that oracle program and write it with the synthetic account
   pattern (`templates/README.md`, pattern 4).
4. Mark environment actions with `LastKind::Env`. Skip 3.9 and 3.10 after them, but refresh all
   baselines.

### 4.5 Per-asset fee and conservation checks

- For each asset in and out of an operation, compute the expected amount from the pre-state
  (FeeCheck pattern, 3.8). Compare each asset separately in its smallest unit.
- Check 3.3 (accounting against real balance) for each asset that the program accounts.
- Wealth (3.9) sums all assets of the signer at the fair prices of 4.2.

---

## 5. Check design rules

These rules come from a large campaign with tolerance 0. They prevented or
removed false positives. Each rule has a lesson in `lessons.md`.

### 5.2 Exact model forms

Use 2 model kinds for each fee, price or share formula:

- **Port model.** A copy of the program formula, in the program's rounding order. Use it for
  agreement checks (amounts, error codes) and for boundary inputs. It finds state and harness
  defects. It cannot find a defect in the formula itself, because it repeats the formula.
- **Docs model.** The formula from the docs or the spec, with each rounding in the direction that
  favors the protocol (fees and prices for the user round up, payouts and minted shares round
  down). Use it for bound checks.
- Unit-test both models against each other over the input domain. Each difference is a lead:
  record it before any run.
- **Rounding table.** The docs model covers top-level formulas. Intermediate steps (a fee
  converted to basis points, a time progress scaled to a multiplier, a cast after a multiply) can
  round in favor of the user inside a formula that looks correct. List every rounding site on a
  value path with its direction and who gains. Each site where the user gains gets a check that
  can fire: a docs model bound with the protocol-favoring direction, or a split flow (3.11) when
  small inputs make the step round to 0.

1. Test each model with randomized unit tests over small supplies and fractional pending values
   (K85).
2. Compute boundary inputs and "fails with code X" claims at the post-setup state, with the full
   fee and each minimum on the amount the program checks (K84, K86). Capture the setup values
   again when the setup changes (K58).
3. Model the rounding of each external program that moves tokens (K88).
4. A property that combines 2 model results can pass by arithmetic. Check it for independence
   from the checks that compare each model with the chain (K67).

### 5.3 Error code agreement

1. A strict code clause must follow the full guard order of the handler, also guards that another
   check owns. Reuse the predicate of the owning check (K69).
2. A failure-side agreement check skips actions that sent no transaction, and applies the charge
   that the failed transaction reverted (K65).
3. Compute codes with a parser that accepts an explicit enum discriminant (K73,
   `crucible-pitfalls.md`, T7).

### 5.4 Frame and write-set checks

1. A multi-op action that stops at its first failure changes state while `success` is false.
   Record each op result (K63).
2. List the setter side effects of each config field (window settle, timestamp) and give the
   write-set check their expected values (K71).
3. A check that reads an action fact must know which actions set it. Compute the fact from the
   state for the other actions (K70).

### 5.5 Wealth with permissionless actions

- If an action is permissionless and its fee mints shares, a signer that holds shares gains. Write
  a named skip for this case (K68).

### 5.6 Liveness of each check

1. Build each canary from a rule that fires on many draws, not from a boundary equality (K66).
2. Give every check a success reach line; an attempt or skip line proves nothing (K91).
3. A rare path of 3 or more steps gets a deterministic setup smoke that calls the check, and the
   canary fires through the smoke (K59).
4. Judge 2-step lines on runs of 120 s or more and 3-step lines on a stateful run (K98, K102).
