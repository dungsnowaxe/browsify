## JavaScript tooling

- Use Node.js 24.22.1 (see `.node-version`).
- Use pnpm for `apps/landing`, with the version pinned in its `package.json`. From the repo root, run `pnpm -C apps/landing install --frozen-lockfile` and `pnpm -C apps/landing build`. Keep its `pnpm-lock.yaml` up to date; do not generate an npm lockfile there.
- The desktop UI in `apps/browsify-ui` still uses its existing npm lockfile and CI commands.

## Agent skills

### Issue tracker

Issues live in GitHub Issues. Before reading or publishing tickets, read `docs/agents/issue-tracker.md`.

### Triage labels

Use the five default triage labels. Before triaging, read `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout. Before exploring the codebase, read `docs/agents/domain.md`.

### macOS testing

Before live browser/profile launches, daemon tests, or default-browser changes, read `.agents/skills/browsify-macos-testing/SKILL.md`. Never launch browsers with a temporary `HOME`; verify both effective default handlers and their restoration rather than trusting API return codes.
