//! TEMPLATE. Generic fuzzed account choice and crash bucketing for a Crucible harness.
//! It contains the first-wins fix of `report_violation` (lessons K33, pitfall C1).
//! Tested with crates.io crucible-* 0.3.0-alpha.1, LiteSVM 0.15.2, anchor-lang 1.0.1.
//!
//! PARTS TO ADAPT for a new target:
//! 1. `Kind::External` and `Filter::External`: an example kind for an account that an
//!    external program owns (synthetic data). Replace them with the external account
//!    kinds of the target (for example an oracle kind), or delete them. Keep
//!    `Filter::matches` and `Entry::kind_name` in step with `Kind`.
//! 2. `print_pool_sizes`: the filter list names `Pda("Pool")` and `Pda("Vault")`,
//!    example IDL account type names. Put the IDL type names of the target there
//!    (one line per `Filter` that a slot uses).
//! 3. `SLOT_RANGE` / `HONEST_CUTOFF` (512 / 384): size them from the setup plan, so that
//!    `SLOT_RANGE - HONEST_CUTOFF` >= the largest pool, and keep 75% honest (K21, K22).
//!    The `#[range(0..N)]` of every slot parameter in main.rs must use the same N.
//! 4. `add_shadow_mints`: takes an explicit mint list. Choose which honest mints get
//!    shadows; each shadow mint adds 1 + owners.len() entries to the registry (K19, K22).
//! The rest (Registry, pick, ActionTag, report_violation, print_map) is generic.
//!
//! Required fixture fields in main.rs: `last_tag: ActionTag`,
//! `tainted: Vec<&'static str>`, `violation_recorded: bool` (reset only by the
//! per-iteration fixture clone; never reset it inside the invariant function).
//! Usage: templates/README.md.
//!
//! - `Registry`: every account that setup() creates or derives, with a kind and an origin.
//! - `Registry::pick`: maps a fuzzed value to an account for one instruction slot.
//!   Soft bias: 75% of the values pick the honest account; every other
//!   type-valid entry stays reachable.
//! - `add_shadow_mints`: for each mint in an explicit list, a look-alike mint plus one
//!   token account of it for each owner in an explicit list (consistent pairs that
//!   pass local checks).
//! - `report_violation`: crash bucketing at report time. Executions are never skipped.

use crucible_fuzzer::{AccountBuilderBase, TestContext};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

/// Fuzzed slot values are in `0..SLOT_RANGE`. Use `#[range(0..512)]` on slot parameters.
/// 512 (not 256): one reference target had 85 entries (84 others) in the `Any` pool.
pub const SLOT_RANGE: usize = 512;
/// Values below this pick the honest account: 384 / 512 = 75%.
pub const HONEST_CUTOFF: usize = 384;

/// Deterministic keypair, so addresses are the same in every run and replay.
pub fn fixed_keypair(tag: u8) -> Rc<Keypair> {
    Rc::new(Keypair::new_from_array([tag; 32]))
}

/// What an account is, by data layout (not by address).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Wallet,
    /// Program-owned account with an Anchor type (IDL account name).
    Pda(&'static str),
    Mint,
    TokenAccount { mint: Pubkey, owner: Pubkey },
    /// Account owned by a supported external program (synthetic data).
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Created by the protocol flow in setup().
    Honest,
    /// Look-alike that an attacker can create on mainnet.
    Shadow,
}

/// Which registry entries are type-valid for a slot (from the slot's declared type).
#[derive(Clone, Copy, Debug)]
pub enum Filter {
    /// `UncheckedAccount` / `AccountInfo`: any key.
    Any,
    /// `Account<'info, T>`
    Pda(&'static str),
    /// `InterfaceAccount<Mint>` / `Account<Mint>`
    Mint,
    /// `InterfaceAccount<TokenAccount>` / `Account<TokenAccount>`
    TokenAccount,
    /// `AccountInfo` with an owner constraint = a supported external program.
    External,
}

impl Filter {
    fn matches(&self, kind: &Kind) -> bool {
        match (self, kind) {
            (Filter::Any, _) => true,
            (Filter::Pda(t), Kind::Pda(k)) => t == k,
            (Filter::Mint, Kind::Mint) => true,
            (Filter::TokenAccount, Kind::TokenAccount { .. }) => true,
            (Filter::External, Kind::External) => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct Entry {
    pub label: String,
    pub address: Pubkey,
    pub kind: Kind,
    pub origin: Origin,
}

/// Append-only: indices must stay stable, or saved inputs select other accounts.
#[derive(Clone, Default)]
pub struct Registry {
    entries: Vec<Entry>,
    /// Wallets the fuzzer may use as signers (not the relayer).
    actors: Vec<Rc<Keypair>>,
}

impl Registry {
    pub fn add(&mut self, label: impl Into<String>, address: Pubkey, kind: Kind, origin: Origin) -> Pubkey {
        self.entries.push(Entry { label: label.into(), address, kind, origin });
        address
    }

    /// A wallet the fuzzer may choose as signer.
    pub fn add_actor(&mut self, label: impl Into<String>, kp: Rc<Keypair>) {
        self.add(label, kp.pubkey(), Kind::Wallet, Origin::Honest);
        self.actors.push(kp);
    }

    pub fn actors(&self) -> &[Rc<Keypair>] {
        &self.actors
    }

    /// Actor for a fuzzed signer index.
    pub fn actor(&self, idx: usize) -> Rc<Keypair> {
        self.actors[idx % self.actors.len()].clone()
    }

    pub fn mints(&self, origin: Origin) -> Vec<Pubkey> {
        self.entries
            .iter()
            .filter(|e| e.kind == Kind::Mint && e.origin == origin)
            .map(|e| e.address)
            .collect()
    }

    /// The token account of `mint` owned by `owner`, if the registry has one.
    pub fn token_account(&self, mint: Pubkey, owner: Pubkey) -> Option<Pubkey> {
        self.entries.iter().find_map(|e| match e.kind {
            Kind::TokenAccount { mint: m, owner: o } if m == mint && o == owner => Some(e.address),
            _ => None,
        })
    }

    /// Every registered token account whose token owner is `owner`, as (mint, address).
    pub fn token_accounts_owned_by(&self, owner: Pubkey) -> Vec<(Pubkey, Pubkey)> {
        self.entries
            .iter()
            .filter_map(|e| match e.kind {
                Kind::TokenAccount { mint, owner: o } if o == owner => Some((mint, e.address)),
                _ => None,
            })
            .collect()
    }

    /// Size of each slot pool. Each pool must have at most SLOT_RANGE - HONEST_CUTOFF
    /// other entries, or some candidates are unreachable.
    pub fn print_pool_sizes(&self) {
        let filters: [(&str, Filter); 6] = [
            ("Any", Filter::Any),
            ("Pda(Vault)", Filter::Pda("Vault")),
            ("Pda(Vault)", Filter::Pda("Vault")),
            ("Mint", Filter::Mint),
            ("TokenAccount", Filter::TokenAccount),
            ("External", Filter::External),
        ];
        let limit = SLOT_RANGE - HONEST_CUTOFF;
        for (name, f) in filters {
            let n = self.entries.iter().filter(|e| f.matches(&e.kind)).count();
            // +1: the honest entry of the slot is not among the "others".
            let flag = if n > limit + 1 { "  [WARN] above limit" } else { "" };
            eprintln!("[SETUP] pool {name:<14} {n:>3} entries (limit {limit} others){flag}");
        }
    }

    fn origin_of(&self, address: &Pubkey) -> Option<Origin> {
        self.entries.iter().find(|e| e.address == *address).map(|e| e.origin)
    }

    /// Choose the account for one slot (soft bias).
    ///
    /// `idx` is a fuzzed value in `0..SLOT_RANGE`:
    /// - `idx < HONEST_CUTOFF` (75%) -> the honest account, if there is one;
    /// - otherwise -> `rest[(idx - HONEST_CUTOFF) % rest.len()]`, where `rest` is
    ///   every other type-valid entry in registry order.
    /// Every entry stays reachable as long as `rest.len() <= SLOT_RANGE - HONEST_CUTOFF`
    /// (128); a larger pool prints a warning because some entries become unreachable.
    /// If there is no honest account (for example the signer has no token account
    /// of this mint), the whole range maps onto `rest`.
    /// The relation to the honest account (honest / other / shadow) is recorded in
    /// `tag` for crash bucketing.
    pub fn pick(
        &self,
        tag: &mut ActionTag,
        slot: &'static str,
        filter: Filter,
        honest: Option<Pubkey>,
        idx: usize,
    ) -> Pubkey {
        let rest: Vec<Pubkey> = self
            .entries
            .iter()
            .filter(|e| filter.matches(&e.kind) && Some(e.address) != honest)
            .map(|e| e.address)
            .collect();
        let deviation_values = SLOT_RANGE - HONEST_CUTOFF;
        if rest.len() > deviation_values {
            eprintln!("[WARN] slot {slot}: {} candidates > {deviation_values} deviation values; some are unreachable", rest.len());
        }

        let chosen = match honest {
            Some(h) if idx % SLOT_RANGE < HONEST_CUTOFF || rest.is_empty() => h,
            Some(_) => rest[(idx % SLOT_RANGE - HONEST_CUTOFF) % rest.len()],
            None => rest[idx % rest.len()],
        };
        let relation = if Some(chosen) == honest {
            "honest"
        } else if self.origin_of(&chosen) == Some(Origin::Shadow) {
            "shadow"
        } else {
            "other"
        };
        tag.slots.push((slot, relation));
        chosen
    }

    /// Role -> address table, for triage of findings that name addresses.
    pub fn print_map(&self) {
        eprintln!("[SETUP] account registry ({} entries):", self.entries.len());
        for (i, e) in self.entries.iter().enumerate() {
            eprintln!("    {i:>2} {:<28} {:?} {:?} {}", e.label, e.origin, e.kind_name(), e.address);
        }
    }
}

impl Entry {
    fn kind_name(&self) -> String {
        match &self.kind {
            Kind::Wallet => "Wallet".into(),
            Kind::Pda(t) => format!("Pda({t})"),
            Kind::Mint => "Mint".into(),
            Kind::TokenAccount { .. } => "TokenAccount".into(),
            Kind::External => "External".into(),
        }
    }
}

/// For each mint in `mints`: a shadow mint with the same decimals, plus one token
/// account of it for each owner in `owners` with `balance`. The last owner is the
/// shadow mint authority. Addresses come from `fixed_keypair(*next_tag)`, and
/// `next_tag` is advanced.
pub fn add_shadow_mints(
    reg: &mut Registry,
    ctx: &mut TestContext,
    mints: &[Pubkey],
    owners: &[Pubkey],
    balance: u64,
    next_tag: &mut u8,
) {
    for honest_mint in mints {
        // SPL Mint layout: COption<Pubkey> (36) + supply u64 (8) + decimals u8
        let decimals = ctx.get_account(honest_mint).expect("mint must exist").data[44];

        let shadow_mint = fixed_keypair(*next_tag).pubkey();
        *next_tag += 1;
        ctx.create_mint()
            .pubkey(shadow_mint)
            .mint_authority(owners[owners.len() - 1])
            .decimals(decimals)
            .supply(balance * owners.len() as u64)
            .create()
            .unwrap();
        reg.add(format!("shadow_of_{}", &honest_mint.to_string()[..4]), shadow_mint, Kind::Mint, Origin::Shadow);

        for owner in owners {
            let ata = fixed_keypair(*next_tag).pubkey();
            *next_tag += 1;
            ctx.create_token_account()
                .pubkey(ata)
                .mint(shadow_mint)
                .token_owner(*owner)
                .amount(balance)
                .create()
                .unwrap();
            reg.add(
                format!("shadow_ata_{}_{}", &shadow_mint.to_string()[..4], &owner.to_string()[..4]),
                ata,
                Kind::TokenAccount { mint: shadow_mint, owner: *owner },
                Origin::Shadow,
            );
        }
    }
}

/// What the last action did, for crash bucketing.
#[derive(Clone, Default, Debug)]
pub struct ActionTag {
    pub action: &'static str,
    pub slots: Vec<(&'static str, &'static str)>,
}

impl ActionTag {
    pub fn new(action: &'static str) -> Self {
        Self { action, slots: Vec::new() }
    }

    fn key(&self) -> String {
        let slots: Vec<String> = self
            .slots
            .iter()
            .filter(|(_, rel)| *rel != "honest")
            .map(|(s, rel)| format!("{s}={rel}"))
            .collect();
        format!("{} [{}]", self.action, slots.join(","))
    }
}

static SEEN_BUCKETS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Report an invariant violation once per bucket (per process).
///
/// Bucket = (invariant, action, relation of each non-honest slot).
/// `tainted` holds the invariants that already failed in this iteration: a
/// state invariant (for example supply, access control or immutability) stays
/// broken after the first failure, and later actions would otherwise create new buckets for the same bug.
/// `recorded` tracks whether Crucible has recorded a violation in this iteration
/// (Crucible keeps only the first violation per iteration). A bucket is consumed
/// from SEEN_BUCKETS only when its message is actually recorded by Crucible.
pub fn report_violation(invariant: &'static str, tag: &ActionTag, tainted: &mut Vec<&'static str>, recorded: &mut bool, msg: String) {
    if tainted.contains(&invariant) {
        return;
    }
    tainted.push(invariant);

    if *recorded {
        return;
    }

    let key = format!("{invariant} | {}", tag.key());
    let seen = SEEN_BUCKETS.get_or_init(|| Mutex::new(HashSet::new()));
    if seen.lock().unwrap().insert(key.clone()) {
        crucible_test_context::record_violation(format!("{{{key}}} {msg}"));
        *recorded = true;
    }
}
