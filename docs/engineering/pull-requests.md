# Pull request guidance

A pull request describes the branch as a whole. Its purpose and scope should be
clear even when the branch contains many focused commits.

## Title

Use a concise, outcome-focused title that distinguishes this branch from other
work. A pull request title does not need to repeat the commit type/scope format.
Avoid generic titles such as `updates` or `fixes`.

## Description

Cover the branch's outcome, why it is useful, and how it was checked. Include
architectural, compatibility, or release/publication consequences when they
matter. A useful starting shape is:

```markdown
## Summary
- ...

## Why
- ...

## Validation
- ...

## Risks and follow-up
- ...
```

Keep only the sections that help reviewers understand this change. Report only
validation that was actually performed, and state material unchecked areas or
known risks plainly. Do not use the description as a dump of every commit title
or as authorization to merge or release.
