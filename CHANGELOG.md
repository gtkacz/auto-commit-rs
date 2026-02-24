# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 

### Changed

- 

### Fixed

- 

### Removed

- 

## [1.1.0] - 2026-02-24

### Added

- `cgen update` subcommand to manually update to the latest version
- `ACR_AUTO_UPDATE` configuration flag (defaults to unset; prompts on first run)
- Automatic version checking against GitHub releases on every run
- Auto-update support when `ACR_AUTO_UPDATE=1` (updates silently before proceeding)
- Update warning displayed at the end of output when a newer version is available and auto-update is off
- `cgen prompt` subcommand to print the LLM system prompt without running anything
- `cgen config` now auto-detects git repo: prompts for global vs local scope inside a repo, opens global directly outside one

### Changed

- Staged files display now uses tree-style characters (`├──`, `└──`) instead of bullet points
- Boolean config fields display "enabled"/"disabled" instead of "1 (yes)"/"0 (no)" in the interactive config UI
- Interactive config groups settings into collapsible tree sections (Basic expanded, Advanced collapsed with subgroups)
- `cgen config --global` flag removed; scope selection is now interactive when inside a git repo

## [1.0.0] - 2026-02-23

- Initial release of the tool
