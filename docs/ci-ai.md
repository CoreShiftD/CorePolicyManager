# AI CI/CD Infrastructure

This document describes the setup for AI-assisted workflows in this repository.

## GitHub Environment: `ai-audit`

To provide tighter control over AI API secrets and execution, all AI workflows are bound to a protected GitHub Environment named `ai-audit`.

### Setup Instructions

1.  Navigate to **Settings** → **Environments** → **New environment**.
2.  Name the environment: `ai-audit`.
3.  Add the following **Environment secrets**:
    -   `GEMINI_API_KEY`: Your Google Gemini API key.

### Recommended Protections

-   **Required reviewers**: Enable this to manually approve any workflow that uses AI secrets.
-   **Branch restrictions**: Restrict use to specific branches if necessary.
-   **Wait timer**: Optional delay before execution.

## Workflow Behavior

-   **Triggers**: Workflows are triggered manually (`workflow_dispatch`) and on a scheduled 6-hour cron (`0 */6 * * *`).
-   **Agentic Editing**: Workflows use the **Gemini CLI** in `auto-edit` mode. Instead of generating text patches, the AI agent directly modifies source files in the checked-out branch.
-   **Model Selection**: Manual runs allow choosing between `auto`, `flash`, `pro`, and `flash-lite`. Scheduled runs default to `flash`.
-   **Strict Safety Gates**:
    -   **Max 1 file** changed per pass.
    -   **Max 50 total lines** changed.
    -   Changes are restricted to specific allowed source paths.
    -   Unsafe paths (secrets, binaries, build artifacts) are automatically rejected and reverted.
-   **Validation Gating**: Every edit is validated via a full build/test suite (`cargo test`, `assembleDebug`, etc.) before being committed.
-   **Daily Branches**: All improvements are committed to isolated **daily branches** (e.g., `ai/rust-YYYY-MM-DD`) for manual review.
-   **Artifacts**: Raw AI output and the generated `git-diff.patch` are uploaded as job artifacts for every run, including failures.
