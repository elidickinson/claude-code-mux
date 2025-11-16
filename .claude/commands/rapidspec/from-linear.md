---
name: RapidSpec: From Linear
description: Convert Linear issue to RapidSpec proposal
category: RapidSpec
tags: [rapidspec, linear, import]
allowed-tools: Read, Write, Edit, Bash, WebSearch, Task
argument-hint: <issue-id-or-url>
---

<!-- RAPIDSPEC:START -->
# Create Proposal from Linear Issue

<command_purpose>
Convert Linear issues to RapidSpec proposals with automated research and verification.
Fetch issue details, run parallel agents for research, present options, generate files.
</command_purpose>

<issue_id> #$ARGUMENTS </issue_id>

<critical_requirement>
MUST fetch full Linear issue details before proceeding.
MUST analyze requirements and map to RapidSpec format.
MUST delegate to /rapid:proposal for complete proposal workflow.
NEVER duplicate proposal logic - use existing workflow.
</critical_requirement>

## Main Tasks

### 1. Fetch Linear Issue (ALWAYS FIRST)

<thinking>
Fetch complete Linear issue data including description, comments, and links.
Parse requirements and acceptance criteria from issue content.
</thinking>

**Fetch process:**
   ```bash
   # Via Linear API
   linear issue <issue-number>
   ```
   - Get title, description, comments
   - Get linked issues and PRs
   - Get current status, assignee, labels
   - Extract acceptance criteria from description

### 2. Analyze Requirements

<thinking>
Parse Linear issue content to extract requirements and constraints.
Map to RapidSpec format (Why, What, Impact).
</thinking>

**Analysis process:**
   - Parse user requirements from description
   - Identify acceptance criteria (bullet points, numbered lists)
   - Extract technical constraints from comments
   - Map to RapidSpec format (Why, What, Options)
   - Prepare feature description for proposal command

### 3. Delegate to Proposal Command

<thinking>
Delegate the complete proposal workflow to /rapid:proposal.
This ensures consistent proposal generation with full research and verification.
</thinking>

**Delegation process:**
   - Execute: `rapid proposal <change-id>`
   - Pass analyzed requirements as feature description
   - Let /rapid:proposal handle all research, verification, and file generation
   - Wait for proposal completion

### 4. Complete

<thinking>
Linear to RapidSpec conversion completed successfully.
</thinking>

**Final steps:**
   - Confirm proposal files created in `rapidspec/changes/<change-id>/`
   - Verify all proposal documents generated (proposal.md, tasks.md, etc.)
   - Ready for implementation with `/rapid:apply <change-id>`

## Linear → Spec Mapping

| Linear Field | Maps To |
|-------------|---------|
| Title | Change title |
| Description | Why section |
| Acceptance Criteria | Requirements |
| Comments | Investigation notes |
| Labels | Spec tags |
| Linked Issues | Related changes |
| Assignee | (preserved in Linear) |

**Examples**

Example 1: Feature Request
```
User: "/rapid:from-linear 123"

AI:
Fetching Linear issue #123...
✓ Found: "[Feature] Add two-factor authentication"
✓ Status: In Progress
✓ Assignee: @chanhee
✓ Description: 850 chars
✓ Acceptance criteria: 3 items

Analyzing requirements...
Main goal: Add 2FA for enhanced security
Requirements:
  - TOTP support (authenticator apps)
  - QR code generation
  - Backup codes (8 codes)
Acceptance criteria:
  1. Users can enable 2FA in settings
  2. QR code shown for initial setup
  3. Backup codes generated and downloadable

Suggested change-id: add-two-factor-auth
Proceed? (yes or custom name)

User: "yes"

AI:
Delegating to /rapid:proposal...
✓ Executing: rapid proposal add-two-factor-auth
✓ Creating proposal with full research and verification...
✓ Generated proposal files:
  - proposal.md
  - investigation.md
  - research.md
  - tasks.md
  - specs/auth/spec.md

Done! Next steps:
- Review: rapidspec/changes/add-two-factor-auth/proposal.md
- Implement: /rapid:apply add-two-factor-auth
```

Example 2: Bug Report
```
User: "/rapid:from-linear https://linear.app/team/issue/BUG-456"

AI:
Fetching Linear issue BUG-456...
✓ Found: "[Bug] 409 token refresh race condition"
✓ Description: "Users see 409 errors when refreshing in multiple tabs"
✓ Comments: 5 (includes reproduction steps)

Analyzing requirements...
Problem: Token refresh race condition
Reproduction: Open app in 2 tabs → 409 error
Expected: Graceful mutex handling

Suggested change-id: fix-token-refresh-race
Proceed? (yes)

User: "yes"

AI:
Delegating to /rapid:proposal...
✓ Executing: rapid proposal fix-token-refresh-race
✓ Running complete proposal workflow with research and verification...
```

Example 3: Incomplete Issue
```
User: "/rapid:from-linear 789"

AI:
Fetching Linear issue #789...
✓ Found: "Add search"
⚠️  Description is empty (3 chars)
⚠️  No acceptance criteria

Issue incomplete. Options:
1. Add details in Linear first (recommended)
2. Create minimal proposal (needs manual editing)

Which? (1 or 2)

User: "1"

AI:
Opening Linear issue in browser...
Please add:
- Description: What should search do?
- Acceptance criteria: How will we know it works?

Run /rapid:from-linear 789 again when ready.
```

**Error Handling**

Issue not found:
```
❌ Linear issue #123 not found
- Check issue number
- Verify LINEAR_API_KEY is set: echo $LINEAR_API_KEY
- Ensure issue is in configured team
```

API error:
```
❌ Linear API error: Unauthorized
Fix:
1. Get API key: https://linear.app/settings/api
2. Set: export LINEAR_API_KEY=lin_api_...
3. Retry: /rapid:from-linear 123
```

**Anti-Patterns**

❌ Don't: Duplicate proposal workflow logic
```
Bad: Implement separate proposal logic in from-linear
→ Code duplication, maintenance overhead
```

✅ Do: Delegate to existing proposal workflow
```
Good: Parse Linear → Delegate to /rapid:proposal
→ Reuse existing, well-tested proposal workflow
```

**Reference**
- Linear API: Auto-fetch via issue number or URL
- Change-id: Kebab-case from title (e.g., "add-two-factor-auth")
- Delegation: Uses `/rapid:proposal` for complete workflow
- User says "yes" (go), "wait" (wait), "no" (no)
- After proposal generation, suggest `/rapid:apply <change-id>`

<!-- RAPIDSPEC:END -->
