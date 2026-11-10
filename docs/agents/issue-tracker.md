# Issue tracker: GitHub

Issues and specs live in GitHub Issues for `dungsnowaxe/browsify`.
Use the `gh` CLI from this repo.

## Conventions

- Before the first issue creation in a session, verify the target with
  `gh repo view --json nameWithOwner`. Confirm with the user if it differs
  from `dungsnowaxe/browsify`.
- Create: `gh issue create --title "..." --body-file <file>`.
- Read: `gh issue view <number> --json number,title,body,labels,comments`.
- List: `gh issue list --state open --json number,title,body,labels`.
- Comment: `gh issue comment <number> --body "..."`.
- Label: `gh issue edit <number> --add-label "..."` or `--remove-label "..."`.
- Close: post an explanatory comment, then `gh issue close <number>`.
- Use native sub-issues and issue dependencies when supported by the
  installed CLI and repository. Otherwise put `Part of #<parent>` and
  `Blocked by: #<number>` at the top of child issue bodies.
  A ticket is unblocked when every blocker is closed.

## Pull requests as a triage surface

**PRs as a request surface: no.**

## Skill operations

“Publish to the issue tracker” means create a GitHub issue.
“Fetch the relevant ticket” means read the GitHub issue.

## Wayfinding operations

- Map: one issue labelled `wayfinder:map`, with Notes,
  Decisions-so-far, and Fog sections.
- Child tickets: link to the map as sub-issues; otherwise use a task list
  in the map and `Part of #<map>` in each child.
- Type labels: `wayfinder:research`, `wayfinder:prototype`,
  `wayfinder:grilling`, or `wayfinder:task`.
- Frontier: first open child in map order without an open blocker
  or assignee.
- Claim: `gh issue edit <number> --add-assignee @me`.
- Resolve: comment with the answer, close the child, then append a
  summary and link to the map's Decisions-so-far.
