# Use one TOML file for routing and preferences

Browsify will use `config.toml` as its single Primary Config so users can inspect and edit the same rules and preferences that the UI saves. A save succeeds when the file is written and the running router has loaded it.

## Editing

Tabs in the same browser profile and UI session share one Config Draft in browser local storage. Closing a tab leaves the draft available in the others. Closing the last tab with unsaved edits prompts the user to stay or discard them. Saving from any tab clears the saved draft across those tabs once reload succeeds. A failed save preserves the draft and the last working routing configuration.

For concurrent edits from tabs or an external editor, merge independent changes and show overlapping changes with their location and their original, draft, and current values. Hand-written TOML rules need no permanent IDs. When deletion, duplication, or reordering makes a match ambiguous, show the affected rules in context for the user to resolve.

## Migration

Browsify will offer to import a detected Finicky config and will also offer migration to existing Browsify users whose rules or preferences span multiple files. The importer will preview the effective settings and ordered rules, count successful and failed rule conversions, and provide a copyable report identifying each failed rule in its source and explaining why it needs manual attention. Users choose a replacement for an unconvertible default browser and approve the result before activation.

JSON, JavaScript, and TypeScript become import formats. QuickJS remains useful for inspecting JS/TS during import. This supersedes the live JavaScript routing decision in [ADR 0001](0001-hybrid-lowering-with-ephemeral-quickjs.md).
