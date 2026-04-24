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
-   **Model Selection**: Manual runs allow choosing between `auto`, `flash`, `pro`, and `flash-lite`. Scheduled runs default to `flash`.
    -   `auto` / `flash`: Uses `gemini-1.5-flash` for fast, cost-effective maintenance.
    -   `pro`: Uses `gemini-1.5-pro` for deeper reasoning.
    -   `flash-lite`: Uses `gemini-1.5-flash-8b`.
-   **Fallback Logic**: If `pro` is requested but fails (e.g., quota exceeded), the workflow automatically falls back to `flash` to ensure the audit completes.
-   **Conservative Maintenance**: Gemini is strictly instructed to act as a **low-risk maintenance bot**.
-   **Strict Patch Limits**: Automated patches are gated by the following constraints:
    -   **Max 3 files** changed.
    -   **Max 120 total lines** (additions + deletions) changed.
-   **Validation Gating**: Every patch is validated via `git apply --check` and a full build/test suite before being committed.
-   **No Direct Push to Main**: All improvements are committed to isolated **daily branches** (e.g., `ai/rust-YYYY-MM-DD`) for manual review.
-   **Cost Awareness**: Gemini API quota and costs may apply.
