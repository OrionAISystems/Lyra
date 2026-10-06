# Commit conventions

Use commit messages as a durable record of one coherent change. The message
format is independent of how Git receives it: `-m`, `-F`, an editor, or another
interface must all produce the same message structure.

## Subject

Use one of these forms:

```text
type(scope): concise summary
type: concise summary
```

The scope names the primary subsystem or component. Omit it when no useful
scope applies; do not invent a scope just to fill the parentheses. Use concise,
action-oriented summaries that describe the result. Avoid quotation marks,
trailing periods, and vague subjects such as `update code` or `fix issues`.
Aim for 72 characters or fewer when clarity allows.

For an incompatible external change, mark it with `!` and explain the impact in
the body, for example `feat(api)!: replace the session response contract`.

## Types

| Type | Use for |
| --- | --- |
| `feat` | A new capability or material expansion of an existing capability |
| `fix` | Incorrect behavior or a bug |
| `refactor` | Structural change intended to preserve behavior |
| `polish` | Substantial UI, UX, styling, interaction, copy, or presentation refinement |
| `perf` | Performance or resource-efficiency improvements |
| `test` | Test, fixture, or benchmark changes |
| `docs` | Documentation changes |
| `build` | Build, packaging, compilation, or development-tooling changes |
| `ci` | Continuous integration and delivery automation |
| `chore` | Repository maintenance that does not fit another type |
| `security` | Security-specific hardening or remediation |
| `release` | Release preparation; use a versioned subject such as `release: vX.Y.Z` |
| `revert` | An intentional reversal of an earlier change |

Choose a type by the change's purpose, not its size. A large behavior-preserving
rewrite is usually `refactor`; a small change can be `feat` if it introduces a
new capability. Use repository-specific scope vocabulary. Lyra scopes may name
areas such as `audio`, `transcription`, `runtime`, `settings`, `updater`,
`themes`, or `ui`; these are examples, not a required registry.

## Body

A title alone is enough for a small, self-explanatory change. For a non-trivial
change, include a short body with:

```text
What changed:
- ...

Why:
- ...

Validation:
- ...
```

Explain the change and its reason. Include `Validation` only for checks or
manual verification actually performed. Do not add empty or `N/A` sections, and
do not imply that an unrun check passed. For an incompatible change, include a
`BREAKING CHANGE:` footer describing what consumers must change.

## Commit boundaries

Prefer one understandable conceptual change per commit, so a commit can be
reviewed or reverted as a unit. Commits do not need to be artificially small;
they should have a clear purpose and avoid bundling unrelated work.
