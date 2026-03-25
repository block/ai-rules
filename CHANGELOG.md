# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`ai-rules migrate`** – One-way migration from the ai-rules–managed layout to the [agents.md](https://agents.md/) standard: writes root `AGENTS.md`, moves `ai-rules/skills` and `ai-rules/commands` into `.agents/`, removes generated files, and purges the `ai-rules/` directory. Supports `--nested-depth`, `--dry-run`, and `--force`; prompts for confirmation unless `--force` or `--dry-run`. See [Migration guide](docs/migration.md).

## [1.5.0] - (see GitHub releases)
