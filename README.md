# CODESPECT skills

Public AI skills from the CODESPECT team for smart contract security work: audits, fuzzing and
the tooling around them. Each skill comes from a real audit workflow.

The repository is a Claude Code plugin marketplace. Each plugin holds one or more skills in the
[Agent Skills](https://agentskills.io) format (`SKILL.md` plus reference files), so other agents
that read that format can use them too.

## Install

Add the marketplace once:

```sh
claude plugin marketplace add CODESPECT-security/skills
```

Then install the plugins you want:

```sh
claude plugin install <plugin-name>@codespect
```

Inside a Claude Code session, `/plugin` opens the same menu.

Without the plugin system, copy a skill folder into your personal skills:

```sh
cp -R plugins/<plugin-name>/skills/<skill-name> ~/.claude/skills/
```

## Plugins

### Solana

| Plugin | Version | Description |
|--------|---------|-------------|
| [crucible-fuzzing](plugins/crucible-fuzzing/) | 1.4.0 | Plan, build, run and triage [Crucible](https://github.com/asymmetric-research/crucible) fuzzing campaigns on Solana and Anchor programs |

Each plugin folder has a README with what the skill changes, the test results, the validated tool
versions and the known limits.

## Repository layout

```
.claude-plugin/marketplace.json     marketplace: one entry per plugin
plugins/<plugin-name>/
  .claude-plugin/plugin.json        name, version, description, keywords
  README.md                         what it does, install, tests, limits
  skills/<skill-name>/
    SKILL.md                        entry point that the agent loads
    references/                     material that SKILL.md points to
```

A plugin can also hold `agents/`, `commands/` or `hooks/` next to `skills/` when a skill needs them.

## Add a plugin

1. Create `plugins/<plugin-name>/` with the layout above. Use lowercase names with hyphens.
2. Write the `SKILL.md` front matter with `name` and a `description` that says when to use the
   skill, not what it does.
3. Remove every client name, project name, address and finding detail. Skills here must be
   neutral.
4. Test the skill: run the same task with and without it on a target with known bugs, and
   compare the results.
5. Write the plugin README: what the skill changes, the test results, validated versions, limits.
6. Add 1 entry to `.claude-plugin/marketplace.json` and 1 row to the table above.
7. Run `claude plugin validate .` from the repository root. It must pass.
8. Raise the plugin `version` in `plugin.json` for each published change.

## License

MIT. See [LICENSE](LICENSE). Each plugin is under the same license unless its folder has its own
`LICENSE` file.
