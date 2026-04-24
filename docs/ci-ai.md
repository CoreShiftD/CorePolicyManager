# AI CI/CD Infrastructure

This document describes the setup for AI-assisted workflows in this repository.

## GitHub Environment: `ai-audit`

To provide tighter control over AI API secrets and execution, all AI workflows are bound to a protected GitHub Environment named `ai-audit`.

### Setup Instructions

1.  Navigate to **Settings** → **Environments** → **New environment**.
2.  Name the environment: `ai-audit`.
3.  Add the following **Environment secrets**:
    -   `CODEX_API_KEY`: Your OpenAI/Codex API key.
    -   `GEMINI_API_KEY`: Your Google Gemini API key.
    -   `OPENAI_API_KEY`: (Optional) If separate from Codex.

### Recommended Protections

-   **Required reviewers**: Enable this to manually approve any workflow that uses AI secrets.
-   **Branch restrictions**: Restrict use to specific branches if necessary (though current workflows create isolated `ai/*` branches).
-   **Wait timer**: Optional delay before execution.

## Workflow Behavior

-   **Manual Dispatch**: Workflows are triggered manually (`workflow_dispatch`). Hourly cron schedules have been removed to prevent unexpected API costs.
-   **Cost Awareness**: Note that consumer subscriptions (ChatGPT Plus, Gemini Advanced) do not necessarily include Developer API usage. API keys may require separate billing or credit quotas.
-   **No Direct Push to Main**: All AI-generated improvements and reports are committed to isolated branches (e.g., `ai/app-YYYY-MM-DD`).
-   **Dual-Agent Agreement**: Changes are proposed by both Codex and Gemini and only considered stable if both agents agree on the path and intent.
