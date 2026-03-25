# Feature Specification: Project README

**Feature Branch**: `003-project-readme`
**Created**: 2026-03-25
**Status**: Draft
**Input**: User description: "create a readme stating in a marketing way explaining the advantages of the tool (features, opensource, small footprint, stable, super fast), and with a comprehensive section for contributors"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Potential User Evaluates the Tool (Priority: P1)

A developer or knowledge worker finds the GitHub repository and needs to quickly decide whether this PDF reader is worth trying. They compare it against Acrobat, browser viewers, and other open-source readers. The README should convince them within 30 seconds.

**Why this priority**: First impression determines adoption. A poor README means no downloads regardless of tool quality.

**Independent Test**: Can be fully tested by reading only the README and verifying it answers "what does it do, why is it better, and how do I get it?" without opening any other file.

**Acceptance Scenarios**:

1. **Given** a visitor lands on the GitHub page, **When** they read the top section, **Then** they understand the core value proposition and primary differentiators within 30 seconds.
2. **Given** a visitor scans the features list, **When** they compare against alternatives, **Then** they can identify at least 5 concrete capabilities unambiguously.
3. **Given** a visitor wants to try it, **When** they look for download instructions, **Then** they find a direct link to the latest release binary with no prerequisite knowledge required.

---

### User Story 2 - First-Time Contributor Sets Up the Dev Environment (Priority: P2)

A developer wants to contribute a fix or feature. They need to go from zero to a working local build without asking questions in an issue thread.

**Why this priority**: A contributor who can't build locally in under 15 minutes will give up. This directly determines the contributor funnel.

**Independent Test**: A developer with Rust and Node knowledge but no prior project knowledge should be able to clone, build, and run the app using only the README instructions.

**Acceptance Scenarios**:

1. **Given** a developer clones the repo, **When** they follow the prerequisites section, **Then** they know exactly which tools and minimum versions to install.
2. **Given** prerequisites are installed, **When** they follow the build steps, **Then** the app compiles and launches without additional research.
3. **Given** a developer reads the architecture section, **When** they want to find relevant code, **Then** they understand the Rust/TypeScript split and key source directories.

---

### User Story 3 - Contributor Submits a PR (Priority: P3)

An experienced contributor wants to add a feature or fix a bug. They need to understand contribution norms: branching model, commit style, PR process, code standards.

**Why this priority**: Contributors who don't follow project norms waste maintainer review time. Clear guidelines prevent friction.

**Independent Test**: A contributor can submit a well-formed PR without any back-and-forth on process, purely by following the README contribution guidelines.

**Acceptance Scenarios**:

1. **Given** a contributor finishes a change, **When** they read the contribution section, **Then** they know the expected branch naming, commit message format, and PR checklist.
2. **Given** a contributor runs the documented checks (`cargo clippy`, `tsc --noEmit`), **When** all checks pass, **Then** they can be confident the PR meets baseline quality requirements.

---

### Edge Cases

- What if a user is on Linux or macOS? The README must clearly state Windows-only support and set expectations.
- What if a contributor has an older Rust toolchain? Prerequisites must specify minimum versions.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: README MUST open with a one-sentence value proposition that is compelling and jargon-free.
- **FR-002**: README MUST include a features list covering: PDF viewing, annotations, form filling, digital signature, PDF compression, 20-language i18n, and Windows file association.
- **FR-003**: README MUST quantify key performance and size claims with at least one concrete number (binary size, startup time, compression ratio, or similar).
- **FR-004**: README MUST include at least one screenshot demonstrating the UI.
- **FR-005**: README MUST include a "Download" section with a direct link to the latest GitHub release.
- **FR-006**: README MUST state the open-source license prominently (badge and/or section).
- **FR-007**: README MUST include a Prerequisites section listing exact tool names and minimum versions required to build.
- **FR-008**: README MUST include step-by-step build instructions that produce a runnable binary.
- **FR-009**: README MUST include an Architecture Overview describing the Rust backend / TypeScript frontend split, key source directories, and the Tauri command flow.
- **FR-010**: README MUST include a Contribution Guide covering: branching convention, commit message style, how to run checks, and PR expectations.
- **FR-011**: README MUST clearly state the current platform support (Windows).
- **FR-012**: README MUST include a link to open issues or the "good first issue" label for new contributors.

### Key Entities

- **README.md**: Single Markdown file at the repository root, rendered by GitHub.
- **Release Asset**: The portable `.exe` linked from the download section.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A first-time visitor can identify the top 3 differentiators within 30 seconds of landing on the page.
- **SC-002**: A developer with Rust and Node experience can build and run the app from scratch in under 15 minutes following only the README.
- **SC-003**: The README answers the 5 most common newcomer questions (what, why, install, build, contribute) without requiring external links beyond tool downloads.
- **SC-004**: All performance and size claims are backed by at least one concrete number or comparison.

## Assumptions

- The app currently targets **Windows only** (portable `.exe`).
- "Small footprint" will be quantified from the actual binary size on disk.
- "Super fast" will be expressed as startup time or rendering speed.
- Screenshots will be provided by the project owner before the README is finalized.
- License is MIT unless stated otherwise — to be confirmed before writing the README.
