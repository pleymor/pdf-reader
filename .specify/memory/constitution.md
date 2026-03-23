<!--
SYNC IMPACT REPORT
==================
Version change: (none) → 1.0.0
Status: Initial constitution — created from constitution-template.md

Modified principles: N/A (new file)

Added sections:
  - Core Principles (5 principles)
  - Technology Stack
  - Build & Distribution
  - Governance

Templates reviewed:
  ✅ .specify/templates/plan-template.md — no changes required; Constitution Check
     section already present as a gate
  ✅ .specify/templates/spec-template.md — no changes required
  ✅ .specify/templates/tasks-template.md — no changes required; task phases
     align with principle-driven workflow

Deferred TODOs: None
-->

# PDF Reader Constitution

## Core Principles

### I. Tauri Desktop-First

The application MUST be built exclusively with Tauri targeting Windows x86_64.
No web deployment, no cross-platform builds (macOS/Linux), and no alternative
desktop frameworks (Electron, Qt, etc.) are permitted. All native OS integration
MUST go through Tauri commands implemented in the Rust backend.

**Rationale**: Tauri provides a lightweight, secure desktop runtime with a small
binary footprint on Windows. Constraining to one platform and one framework
reduces complexity and ensures a consistent, well-tested delivery path.

### II. Single Executable Distribution

The build output MUST be a standalone `.exe` file for Windows 64-bit only.
MSI, NSIS, and all other bundle types MUST be disabled in `tauri.conf.json`.
The Rust target triple MUST be `x86_64-pc-windows-msvc`. No side-car DLLs or
external runtime dependencies may be introduced without explicit justification.

**Rationale**: A single `.exe` simplifies distribution, eliminates installer
complexity, and allows users to run the application without admin privileges.

### III. Rust Backend, Web Frontend

Business logic, file I/O, and PDF processing MUST reside in Rust (`src-tauri/`).
The frontend (HTML/CSS/TypeScript) MUST handle presentation and user interaction
only. All cross-boundary calls MUST use Tauri's `invoke` command API. No
sensitive file-system operations or OS calls may originate from the frontend.

**Rationale**: This boundary enforces Tauri's security model (CSP/allowlist),
enables independent unit-testing of core logic, and clarifies responsibility.

### IV. Simplicity & YAGNI

Every feature MUST justify its complexity before implementation begins.
No abstractions for hypothetical future requirements are permitted.
Adding a third-party crate or npm package MUST be explicitly justified in the
relevant feature spec or task. Prefer direct solutions over layered architectures.

**Rationale**: PDF readers are well-understood; over-engineering adds maintenance
burden without delivering user value.

### V. Test-Driven Development

Unit tests for Rust backend functions MUST be written before implementation.
The Red-Green-Refactor cycle MUST be followed: tests MUST fail first, then
implementation is written to make them pass, then code is refactored.
Frontend tests MUST cover each user-facing interaction where behavior is
non-trivial. Tests for a user story MUST be committed before the story is
marked complete.

**Rationale**: PDF parsing has many edge cases; TDD catches regressions early
and provides living documentation of expected behavior.

## Technology Stack

- **Framework**: Tauri v2 (Rust + WebView2 on Windows)
- **Target platform**: Windows x86_64 (`x86_64-pc-windows-msvc`)
- **Frontend language**: TypeScript + HTML/CSS
- **Frontend framework**: Minimal — no heavy JS framework unless justified per spec
- **PDF engine**: Determined per feature spec (candidates: `pdfium-render`, `lopdf`)
- **Build command**: `cargo tauri build --target x86_64-pc-windows-msvc --release`
- **Rust testing**: `cargo test`
- **Frontend testing**: Vitest (unit) or WebdriverIO (e2e), only if tests requested

## Build & Distribution

- `tauri.conf.json` MUST set `bundle.targets` to `["exe"]` exclusively.
- The `productName` field and Windows file metadata MUST reflect "PDF Reader".
- Only release builds (`--release`) are distributed; debug builds are local only.
- Auto-updater configuration is NOT required unless a feature spec mandates it.
- Code signing is OPTIONAL unless explicitly required by a feature spec.
- CI pipelines, if added, MUST produce the `.exe` artifact on a Windows runner.

## Governance

This constitution supersedes all other development conventions for this project.
Amendments MUST be proposed by updating this file with a version bump following
semantic versioning rules:

- **MAJOR**: Principle removal, platform target change, or framework replacement.
- **MINOR**: New principle or section added, or materially expanded guidance.
- **PATCH**: Clarification, wording refinement, or typo fix.

All feature plans (`plan.md`) MUST include a "Constitution Check" section that
verifies compliance with every principle before implementation begins. Any
necessary deviation MUST be recorded in the plan's Complexity Tracking table
with explicit justification for why a simpler compliant approach was rejected.

Compliance review MUST occur at: (1) plan approval, (2) PR review, and
(3) any amendment to this constitution.

**Version**: 1.0.0 | **Ratified**: 2026-03-22 | **Last Amended**: 2026-03-22
