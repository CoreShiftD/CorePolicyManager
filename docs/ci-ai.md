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
-   **Branch restrictions**: Restrict use to specific branches if necessary (though current workflows create isolated `ai/*` branches).
-   **Wait timer**: Optional delay before execution.

## Workflow Behavior

-   **Triggers**: Workflows are triggered manually (`workflow_dispatch`) and on a scheduled 6-hour cron (`0 */6 * * *`).
-   **Job Timeout**: Each job has a strict `timeout-minutes: 35`.
-   **Session Budget**: Each workflow run targets a **30-minute AI session** (`SESSION_SECONDS=1800`). It performs repeated review passes within this budget.
-   **Gemini-Only**: Codex and OpenAI integration has been removed to simplify quota management and focus on Gemini-based reviews.
-   **Report-Only**: Currently, workflows generate reports under `reports/ai/` rather than mutating source code.
-   **No Direct Push to Main**: All AI-generated improvements and reports are committed to **daily branches** (e.g., `ai/app-YYYY-MM-DD`).
-   **Branch Reuse**: Scheduled runs reuse the same daily branch, rebasing it onto `main` to accumulate improvements throughout the day.
-   **Cost Awareness**: Gemini API quota and costs may apply. Manual runs are recommended for specific targeted reviews.
