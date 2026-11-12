# crucible-fuzzing

A Claude Code skill for fuzzing Solana and Anchor programs with
[Crucible](https://github.com/asymmetric-research/crucible) (the `crucible` CLI,
`crucible-test-context`, `crucible-idl-gen` and LiteSVM).

The skill turns a fuzzing campaign into gated stages: intake, design spec, build, runs, triage and
coverage. It supplies the Crucible and LiteSVM facts that a capable agent does not know, and it
requires the parts of a harness that agents leave out when nobody asks for them.

## What it changes

Tested on 2 Anchor programs with known bugs: a small one and a larger one with external program
CPIs. Each test compared runs with and without the skill on the same prompt.

| Without the skill, agents tend to | With the skill, the design has |
|-----------------------------------|--------------------------------|
| Make the acting user pay the transaction fee, so the signer probe skips it | A relayer as fee payer and every authority in `.signers()` |
| Write 1 action per bug they already suspect | A typed account registry with soft bias to the honest account; suspects go to a lead list |
| Report "0 crashes" from checks that never fired | A canary, a success reach line and an expected bucket per check |
| Seed stateful runs with `--corpus-in` | Stateful runs from an empty corpus, pool size per worker sized to RAM |
| Miss that LiteSVM loads p-token as SPL Token | The runtime token program checked at intake |
| Copy the program's own formula into the check | A port model and a docs model per formula, a rounding table and a split-flow check |
| Write 4,000 to 6,500 word specs | A spec within a size limit, with derivations in an appendix |

On the larger program, the share of known issues that the design could detect (a check plus a
path that reaches it) went from 2 of 7 without the skill to 6 of 7 with it. The remaining issue
needs code review.

## Install

As a plugin from the CODESPECT marketplace:

```sh
claude plugin marketplace add CODESPECT-security/skills
claude plugin install crucible-fuzzing@codespect
```

Or copy the skill folder into your personal skills (from the repository root):

```sh
cp -R plugins/crucible-fuzzing/skills/crucible-fuzzing ~/.claude/skills/
```

The skill loads when a task mentions Crucible fuzzing of a Solana or Anchor program. You can also
ask for it by name.

## Contents

Paths are relative to `skills/crucible-fuzzing/`.

| Path | Content |
|------|---------|
| `SKILL.md` | Stages and gates, the 10 required design spec slots, the facts agents get wrong |
| `references/playbook.md` | The 20-step procedure |
| `references/lessons.md` | 104 lessons (K1-K104) by topic, each with rule, reason and input |
| `references/crucible-pitfalls.md` | 44 tool and runtime facts with version tags and source lines |
| `references/invariant-catalog.md` | 11 check classes, valuation rules for vaults that hold several assets, exact model rules |
| `references/templates/` | Account registry and crash bucketing code, and 9 code patterns |

## Validated versions

| Component | Version |
|-----------|---------|
| Crucible CLI and crates | 0.3.0-alpha.1 |
| LiteSVM | 0.15.2 |
| Target programs | Anchor 0.29, 0.30 and 0.31 |

Tool facts carry version tags. Check them again when a version changes.

## Limits

- The lessons come from 3 audit campaigns. A lesson's status line gives the number of targets
  that showed it. Lessons seen on 1 target are less certain.
- The skill makes runs more expensive: in the tests, a design run cost about 1.2 to 2 times as
  much as one without the skill, because the agent reads the references and writes more checks.
- The independent spec review is a separate step that asks the user first. It adds about 10
  minutes and one subagent.

## License

MIT. See [LICENSE](../../LICENSE) at the repository root.
