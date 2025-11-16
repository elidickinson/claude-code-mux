---
name: RapidSpec: Apply
description: Implement approved RapidSpec change and keep tasks in sync
category: RapidSpec
tags: [rapidspec, apply, implementation]
allowed-tools: Read, Write, Edit, Bash, Grep, Glob, Task
argument-hint: <change-id>
---

<!-- RAPIDSPEC:START -->
# Apply RapidSpec Implementation

<command_purpose>
Implement RapidSpec proposals step-by-step with checkpoint-based workflow.
Each task is testable and allows direction changes at any checkpoint.
</command_purpose>

<change_id> #$ARGUMENTS </change_id>

<critical_requirement>
MUST show diffs before every file change. No exceptions.
MUST stop immediately when user says "wait" or "stop".
MUST update tasks.md after completing each task (change `- [ ]` to `- [x]`).
NEVER implement multiple tasks without checkpoints between them.
NEVER mark tasks complete without actually implementing them.
</critical_requirement>

## Environment Setup (Optional)

<thinking>
For isolated development, use git worktree to create a separate working directory.
This keeps main branch clean and allows parallel work on multiple changes.
Skip this if working directly on a feature branch.
</thinking>

**Git Worktree Setup:**

1. **Update Main Branch**
   ```bash
   git checkout main
   git pull origin main
   ```

2. **Create Feature Branch and Worktree**
   ```bash
   # Get git root directory
   git_root=$(git rev-parse --show-toplevel)

   # Create worktrees directory
   mkdir -p "$git_root/.worktrees"

   # Add to .gitignore if not already there
   if ! grep -q "^\.worktrees$" "$git_root/.gitignore"; then
     echo ".worktrees" >> "$git_root/.gitignore"
   fi

   # Create worktree with feature branch
   # Branch name example: feature/add-duplicate-prevention
   git worktree add -b feature/<change-id> "$git_root/.worktrees/<change-id>" main

   # Change to worktree directory
   cd "$git_root/.worktrees/<change-id>"
   ```

3. **Verify Environment**
   ```bash
   # Confirm in correct directory
   pwd
   git branch --show-current

   # Install dependencies if needed
   npm install  # or pnpm install, yarn, etc.

   # Run initial tests to ensure clean state
   npm test
   ```

4. **Cleanup After Merge** (when change is complete)
   ```bash
   # Go back to main directory
   cd "$git_root"

   # Remove worktree
   git worktree remove .worktrees/<change-id>

   # Delete feature branch (if merged)
   git branch -d feature/<change-id>
   ```

## Main Tasks

### 1. Read Spec Files (ALWAYS FIRST)

<thinking>
First, understand what needs to be implemented by reading all spec files.
This provides context for implementation decisions and prevents "imaginary code".
</thinking>

**Immediate Actions:**
- [ ] Read `rapidspec/changes/<change-id>/proposal.md` - Chosen approach
- [ ] Read `rapidspec/changes/<change-id>/tasks.md` - Implementation plan
- [ ] Read `rapidspec/changes/<change-id>/design.md` - Architecture (if exists)
- [ ] Read `rapidspec/changes/<change-id>/investigation.md` - Context (if exists)
- [ ] List all tasks with current status

### 1.5 Architecture Validation (Before Implementation)

<thinking>
Before starting implementation, validate architectural decisions one more time.
This catches design issues early when changes are cheap, before any code is written.
Provides concrete implementation guidance based on the proposal.
</thinking>

<critical_requirement>
MUST run architecture agents before implementing first task.
MUST validate design decisions align with proposal.
Design feedback here prevents mid-implementation pivots that waste time.
</critical_requirement>

**Run design agents based on proposal content:**

  - Provide concrete implementation guidance
  - Validate RLS policies before writing SQL
  - Confirm index strategy matches query patterns
  - Review migration safety (CONCURRENTLY, non-blocking)
  - Check rollback plan exists

**If Next.js/React changes (check proposal.md for components, routes):**
- Task nextjs-architecture-expert(components, routes, proposal_context)
  - Guide Server/Client Component decisions per file
  - Review data fetching approach (fetch, use, parallel)
  - Validate file structure and naming
  - Check async params/searchParams usage
  - Suggest loading and error boundaries

**Architecture Validation Checklist:**

After agents complete:
- [ ] Implementation approach confirmed
- [ ] Potential pitfalls identified
- [ ] Concrete guidance received for complex tasks
- [ ] Ready to start Task 1.1 with confidence

**Example Architecture Validation:**

```
Reading proposal.md...
✓ Chosen: Option 1 (Client + DB Unique Constraint)
✓ Detected: Database changes + Next.js component changes

Running architecture validation...

@agent-database-architect:
Reviewing migration plan...
✓ Migration approach: CREATE UNIQUE INDEX CONCURRENTLY - correct
💡 Concrete guidance:
  1. Create index first (can run while app is live)
  2. Add constraint second (requires index to exist)
  3. Order matters: index → constraint, not constraint → index
⚠️ Don't forget: Add IF NOT EXISTS for idempotency
✓ RLS: Policy using has_role('user') - validated
✓ Rollback: DROP INDEX CONCURRENTLY documented

@agent-nextjs-architecture-expert:
Reviewing component structure...
✓ Server Component for SmartLinksPage - correct
✓ Client Component for DuplicateToast - minimal, appropriate
💡 Implementation guidance:
  1. Create toast.tsx as "use client" first
  2. Import into page.tsx (Server Component)
  3. Pass error from Server Action as prop
⚠️ Watch out: Don't use useFormStatus in Server Component
✓ Loading: loading.tsx pattern suggested - add to tasks

Ready to implement with guidance ✓
```

**When to skip:**
- Skip if proposal already has detailed design review
- Skip for minor changes (typo fixes, copy updates)
- Skip if confident in straightforward implementation

### 2. Execute Tasks One by One (Checkpoint-Based)

<thinking>
Implement one task at a time with checkpoints.
Show diffs before changes, wait for approval, implement, then checkpoint.
This allows user to test and change direction at any point.
</thinking>

**Per-Task Flow:**
1. **Announce**: "Starting Task 1.1: [Name] (X min)"
2. **Show Context**: Read actual files, check git history
3. **Propose Changes**: Show diff with Before (actual) → After (proposed)
4. **Wait for Approval**: "Ready to implement? (yes to proceed)"
5. **Implement**: Create/modify files with verified changes
6. **Update tasks.md**: Mark task as complete by changing `- [ ]` to `- [x]`
7. **Checkpoint**: "✅ Task 1.1 complete - Test this? (yes to continue)"

3. **Handle Direction Changes**
   - If user says "wait", "no", or "stop":
     1. Stop immediately
     2. Ask: "What should I change?"
     3. Revise current step
     4. Show new diff
     5. Wait for approval again
   - Update tasks.md to reflect approach changes

4. **Verification Before Changes**
   - Before touching any file:
     1. Read the actual file
     2. Show current code (with @file:line)
     3. Show proposed changes
     4. Show diff (+ Added, - Removed, ~ Modified)
     5. Wait for approval to proceed
   - **NEVER** modify files without showing diffs first

5. **Complete Implementation**
   - After all tasks complete, show summary:
     - Files changed
     - Migrations created
     - Tests added
   - Suggest: `/rapid:review` for quality, security, architecture review
   - Suggest: `/rapid:commit` to create commit with conventional format

**Task Structure**

tasks.md format:
```markdown
## 1. Implementation

### 1.1 [Step Name] (X min) - Checkpoint ⏸
- [ ] [specific task]
- [ ] [specific task]
**Checkpoint:** User can test here

### 1.2 [Next Step] (Y min) - Checkpoint ⏸
- [ ] [specific task]
- [ ] Add E2E test for [feature]
**Checkpoint:** User can test here
```

**Examples**

Example 1: Smooth Flow
```
User: "/rapid:apply add-loading-state"

AI reads spec:
✓ proposal.md: Suspense boundary approach
✓ tasks.md: 3 tasks (15 min total)

AI lists tasks:
1.1 Add Suspense boundary (5 min)
1.2 Create loading.tsx (5 min)
1.3 Test loading state (5 min)

AI: "Ready to proceed?"
User: "yes"

AI starts Task 1.1:
Current: @src/app/dashboard/page.tsx:12
[shows actual code]

Proposed:
[shows code with Suspense]

AI: "Implement these changes?"
User: "yes"

AI: "✅ Task 1.1 complete - Checkpoint ⏸ Continue?"
User: "yes"

AI continues to Task 1.2...
```

Example 2: Direction Change
```
User: "/rapid:apply add-auth-mutex"

AI: "Task 1.1: Install p-limit"
[proposes npm install]


AI: "Understood. Changing approach..."

- Benefit: Built-in mutex (no p-limit needed)
- Breaking: Update client.ts required

AI: "Proceed with upgrade?"
User: "yes"

AI updates tasks.md and implements revised approach
```

Example 3: Found During Implementation
```
AI: "Task 1.2: Add API validation"

Reading @src/app/api/route.ts...
⚠️  Found: Validation already exists at line 89!

Options:
1. Use existing validation (faster)
2. Replace existing (cleaner)

AI: "Which? (1 or 2)"
User: "1"

AI: "Task 1.2 (Revised): Extend existing validation"
[shows diff with existing code]
```

**Error Handling**

If implementation fails:
```
Error: File not found: src/app/api/smart-links/route.ts

Investigating...
✓ Found: src/app/api/smartlinks/route.ts (different path)

Revising task with correct path. Continue?
```

If tests fail:
```
❌ E2E test failed: Expected 400, got 409

Fix options:
1. Change API to return 400 (as spec'd)
2. Update test to expect 409

Which? (1 or 2)
```

**Anti-Patterns**

❌ Don't: Implement all tasks at once
```
Bad: [implements 10 tasks without checkpoints]
→ User can't test incrementally
→ Hard to change direction
```

✅ Do: Step-by-step with checkpoints
```
Good:
Task 1.1 ✓ → Checkpoint ⏸
Task 1.2 ✓ → Checkpoint ⏸
Task 1.3 ✓ → Checkpoint ⏸
```

❌ Don't: Ignore "wait"
```
User: "wait"
Bad: [continues implementing]
```

✅ Do: Stop immediately
```
User: "wait"
Good: "Stopped. What should I change?"
```

**Reference**
- User tests frequently - expect pauses
- Show diffs for every file change
- One task at a time, one checkpoint at a time
- After complete, suggest `/rapid:review` for quality check

<!-- RAPIDSPEC:END -->
