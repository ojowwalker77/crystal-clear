# Changelog

All notable changes to this project are documented in this file.

## [2026-02-09] - TUI Trust + Speed + UX Upgrade

### Added
- Persistent clean operation reporting with summary metrics (`cleaned`, `failed`, `skipped`, `freed`) and failure reason aggregation.
- Full-screen "Last Clean Report" overlay (`v`) showing failed paths and reasons.
- TUI config section (`[tui]`) with defaults for zero-byte filtering, report behavior, and scan parallelism.
- New home-list columns for per-row result state, risk badge (`L/M/H`), and category label.
- Tests covering confirm key mappings, clean report summarization, missing cleaner routing, and zero-byte filtering.

### Changed
- Confirmation flow now explicitly supports `y/Y/Enter` to confirm and `n/Esc` to cancel in modal contexts.
- Cleaned item rows are no longer dropped after cleaning; results remain visible until next scan.
- Home view now includes a compact "Last clean" strip plus richer selection summary with risk composition.
- Footer/help hints now match actual supported key bindings.
- Scan orchestration now runs cleaners with bounded parallel workers while preserving deterministic result ordering.

### Fixed
- Resolved regression where pressing `Y` in the confirmation modal could fail to trigger cleaning.
- Avoided accidental clean/uninstall trigger from `Enter` outside confirmation mode.
- Missing cleaner categories now produce explicit failure rows instead of appearing successful.

### Performance
- Removed artificial scan delay and switched to progress events driven by real cleaner lifecycle updates.
- Improved walker traversal by pruning protected/out-of-bound entries before descent and reducing wasted metadata work.
