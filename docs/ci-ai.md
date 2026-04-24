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
-   **Iterative Sessions**: Each workflow run is a **30-minute AI session**. It performs repeated improvement passes within this budget.
    -   **Active Phase**: New improvements are attempted for the first **25 minutes**.
    -   **Cleanup Phase**: Final **5 minutes** are reserved for final validation and pushing accumulated changes.
-   **Philosophy**: Each run attempts focused, high-value improvements. successful passes are committed separately to maintain a clean history.
-   **Model Selection**: Manual runs allow choosing between `auto`, `flash`, `pro`, and `flash-lite`. Scheduled runs default to `flash`.
-   **Strict Safety Gates**:
    -   **Max 3 files** (or 5 in deep mode) changed per pass.
    -   **Max 180 total lines** (or 300 in deep mode) changed.
    -   Changes are restricted to specific allowed source paths.
    -   Unsafe paths (secrets, binaries, build artifacts) are automatically rejected and reverted.
-   **Validation Gating**: Every pass is validated via its respective test suite before commitment. A final full build check is performed before the session ends.
-   **Daily Branches**: All improvements are committed to isolated **daily branches** (e.g., \`ai/rust-YYYY-MM-DD\`) for manual review.
-   **Artifacts**: Detailed artifacts for every pass (output, patch, status) are retained for 3 days for transparency.

## Commit Policy

AI agents and automated workflows must use **Signed-off-by** commits for traceability and contribution provenance. This ensures compliance with the Developer Certificate of Origin (DCO).

### Standards
- Every automated commit uses \`git commit --signoff\`.
- Agents must include the signoff footer in any git instructions they generate.
- If git identity is missing, agents will instruct the user to configure \`user.name\` and \`user.email\` before proceeding.

