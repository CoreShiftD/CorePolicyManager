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

-   **Triggers**: Workflows are triggered manually (\`workflow_dispatch\`) and on a scheduled 6-hour cron (\`0 */6 * * *\`).
-   **Iterative Maintenance Sessions**: Each workflow run is a **30-minute persistent engineering session**. The AI agent directly modifies source files in the checked-out branch, prioritizing **meaningful progress over trivial churn**.
    -   **Active Phase (25 mins)**: The agent iteratively Inspects, Plans, Edits, Validates, and Commits improvements.
    -   **Strategy Rotation**: If a specific approach (e.g., correctness) produces no changes, the agent automatically rotates to another category (e.g., tests, clippy, error handling).
    -   **Recovery Loop**: If validation fails, the error log is fed back to the agent for up to 2 recovery attempts before reverting.
    -   **Cleanup Phase (5 mins)**: Reserved for final integration checks and pushing accumulated changes.
-   **Commit Policy**: Every automated commit uses **Signed-off-by** for traceability and contribution provenance.
-   **Strict Safety Gates**:
    -   **Max 3 files** (or 5 in deep mode) changed per pass.
    -   **Max 180 total lines** (or 300 in deep mode) changed.
    -   Changes are restricted to specific allowed source paths.
    -   Unsafe paths (secrets, binaries, build artifacts) are automatically rejected and reverted.
-   **Artifacts**: A comprehensive \`session-summary.md\` and detailed pass-by-pass artifacts are retained for 3 days.


