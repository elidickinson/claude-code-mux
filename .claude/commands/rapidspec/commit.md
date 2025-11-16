---
name: RapidSpec: Commit
description: Create well-formatted commits with conventional commit format
category: RapidSpec
tags: [rapidspec, commit, git]
allowed-tools: Read, Bash, Grep, Glob, Task
argument-hint: [message] | --no-verify | --amend
---

<!-- RAPIDSPEC:START -->
# Commit RapidSpec Implementation

<command_purpose>
Create conventional commits with task verification and discovered work capture.
Verify git changes match tasks.md before marking complete, support partial commits.
</command_purpose>

<critical_requirement>
MUST verify claims with `git diff` and `git status` - NEVER trust memory.
MUST match actual git changes to tasks.md before marking complete.
MUST capture discovered work (unplanned improvements).
CAN commit partial work for incremental progress.
</critical_requirement>

## Main Tasks

### 1. Review Git Changes (ALWAYS FIRST)

<thinking>
First, verify what actually changed by reading git status and diff.
Never assume or trust memory - always check actual file modifications.
This prevents marking tasks complete incorrectly.
</thinking>

**Immediate Actions:**
   ```bash
   git status
   git diff
   git log --oneline -5
   ```
   - List all modified, added, deleted files
   - Analyze actual code changes
   - Check recent commit messages for style

### 2. Match Changes to Tasks

<thinking>
Cross-reference git changes with tasks.md to verify completion.
Mark tasks as complete only if git diff confirms the work.
Capture any discovered work not in original task list.
</thinking>

**Task verification process:**
   - Read `rapidspec/changes/<change-id>/tasks.md`
   - For each task, check if actual git changes match:
     - Task 1.2: API Validation → Check API route files
     - Task 1.3: UI Toast → Check component files
   - Mark matching tasks as `[x]`
   - Leave incomplete tasks as `[ ]`
   - Capture discovered work not in original tasks

### 3. Update tasks.md

<thinking>
Update the task file to reflect actual completion state.
Document discovered work for traceability and future reference.
</thinking>

**Actions:**
   - Update checkboxes: `[ ]` → `[x]` for completed tasks
   - Add "Discovered Tasks" section if unplanned work found
   - Show completion status: "5/6 tasks complete"

### 4. Generate Commit Message

<thinking>
Create descriptive conventional commit message that summarizes changes.
Include completed tasks, discovered work, and testing information.
</thinking>

**Format:**
   ```
   type(scope): brief description

   ## Completed Tasks (X/Y)
   - Task description
   - Task description

   ## Additional Improvements (if discovered work)
   - Unplanned improvement
   - Unplanned improvement

   ## Testing
   - E2E: test-file.spec.ts ✓
   - Unit: test.test.ts ✓

   Related: Linear #123 (if linked)

   🤖 Generated with Claude Code
   Co-Authored-By: Claude <noreply@anthropic.com>
   ```

### 5. Wait for Approval

<thinking>
Show commit message preview and wait for user confirmation.
User may want to revise message or change what's committed.
</thinking>

**Actions:**
   - Show commit message preview
   - Ask: "Ready to commit? (yes to proceed)"
   - If user says "wait" or "revise", modify message

### 6. Create Commit

<thinking>
Stage appropriate files and create commit with approved message.
Show commit hash and suggest next steps in workflow.
</thinking>

**Actions:**
   ```bash
   git add <files>
   git commit -m "<message>"
   ```
   - Show commit hash
   - Suggest next steps: `/rapid:review` or `/rapid:archive`

## Commit Types
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code restructuring
- `perf`: Performance improvement
- `test`: Test addition
- `docs`: Documentation
- `chore`: Maintenance
- `style`: Formatting

**Examples**

Example 1: Complete Feature
```
User: "/rapid:commit add-loading-state"

AI reviews git:
✓ Git diff analyzed: 3 files changed
✓ Modified: src/app/dashboard/page.tsx (+12 -3)
✓ Added: src/app/dashboard/loading.tsx (new)
✓ Added: e2e/dashboard/loading.spec.ts (new)

Matching to tasks.md...
✓ Task 1.1: Add Suspense boundary - DONE
✓ Task 1.2: Create loading.tsx - DONE
✓ Task 1.3: Test loading state - DONE

Status: 3/3 tasks complete

Commit message:
────────────────────────────────────────
feat(ui): add loading states with Suspense boundaries

## Completed Tasks (3/3)
- Add Suspense boundary to dashboard page
- Create loading.tsx with skeleton UI
- Test loading state behavior

## Testing
- E2E: dashboard/loading.spec.ts ✓
- Manual testing: verified Suspense works ✓

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>
────────────────────────────────────────

Ready? (yes)

User: "yes"

AI: ✓ Committed: abc1234
```

Example 2: Partial Commit
```
User: "/rapid:commit add-auth-mutex"

AI reviews git:
✓ 2 files changed
✓ Modified: package.json (new dependency)

Matching to tasks.md...
✓ Task 1.2: Update client.ts - DONE
⏳ Task 2.1: Update all auth calls - NOT DONE
⏳ Task 2.2: Add error handling - NOT DONE

Status: 2/4 tasks complete

Options:
1. Commit partial work (2/4 tasks)
2. Complete remaining tasks first

Which? (1 or 2)

User: "1"

AI:
Commit message:
feat(auth): add mutex to prevent token refresh race

Partial implementation (2/4 tasks):
- Update client.ts to use new createBrowserClient

Remaining:
- Update all auth calls to use new client
- Add error handling for mutex timeout

Ready? (yes)
```

Example 3: Discovered Work
```
User: "/rapid:commit prevent-duplicate-smartlinks"

AI reviews git:
✓ 8 files changed

Matching to tasks.md...
✓ 5 planned tasks completed

Discovered work (not in tasks):
📝 Added: Index on release_id for performance
📝 Added: Extracted validation logic to helper
📝 Fixed: Type error in SmartLinkForm

Updating tasks.md...
Added section "4. Performance & Quality (Discovered)"

Commit message:
feat(smart-links): prevent duplicate links per release

## Completed Tasks (5/5)
- API Validation: 400 error handling
- UI Toast: duplicate warning
- E2E Test: duplicate prevention flow
- Unit Test: validation logic

## Additional Improvements
- Performance: Add index on release_id
- Code quality: Extract validateSmartLink()
- Fix: Type error in SmartLinkForm

Related: Linear #124

Ready? (yes)
```

**Anti-Patterns**

❌ Don't: Mark tasks complete without verifying
```
Bad: User says "commit" → Mark all [x] → Commit
→ No git verification
```

✅ Do: Verify with git first
```
Good: git diff → Match to tasks → Mark verified [x]
```

❌ Don't: Lose discovered work
```
Bad: Added index during implementation → Not recorded
```

✅ Do: Capture all work
```
Good: Add "Discovered Tasks" → Update tasks.md
```

**Reference**
- Always run `git diff` and `git status` first
- Update tasks.md with actual completion status
- Conventional commit format: `type(scope): description`
- User says "yes" (go), "wait" (wait), "no" (no)
- Can commit multiple times per spec
- After commit, suggest `/rapid:review` for quality check

<!-- RAPIDSPEC:END -->
