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

-   **Conservative Maintenance**: Gemini 1.5 Flash is used as a **low-risk maintenance bot**. It is not authorized to perform deep architectural refactors.
-   **Strict Patch Limits**: Automated patches are gated by the following constraints:
    -   **Max 3 files** changed.
    -   **Max 120 total lines** (additions + deletions) changed.
    -   No new dependencies or module restructuring.
-   **Patch Protocol**: The model is instructed to return ONLY a unified diff patch. If no safe maintenance improvement is found, it returns `NO_PATCH`.
-   **Validation Gating**: Every patch is validated via `git apply --check` and a full build/test suite (`cargo test`, `assembleDebug`, etc.) before being committed.
-   **Audit Trail**: If no patch is applied or validation fails, a report is committed to the daily branch explaining the outcome.
-   **No Direct Push to Main**: All improvements are committed to isolated **daily branches** (e.g., `ai/rust-YYYY-MM-DD`) for manual review and merging.
-   **Cost Awareness**: Workflows run every 6 hours and manually to control Gemini API quota usage.
