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

-   **Manual Dispatch**: Workflows are triggered manually (`workflow_dispatch`).
-   **Gemini-Only**: Codex and OpenAI integration has been removed to simplify quota management and focus on Gemini-based reviews.
-   **Report-Only**: Currently, workflows generate reports under `reports/ai/` rather than mutating source code.
-   **No Direct Push to Main**: All AI-generated improvements and reports are committed to isolated branches (e.g., `ai/app-YYYY-MM-DD`).
