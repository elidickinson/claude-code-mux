---
name: RapidSpec: Archive
description: Archive deployed RapidSpec change and update specs
category: RapidSpec
tags: [rapidspec, archive]
allowed-tools: Read, Write, Edit, Bash, Task
argument-hint: <change-id>
---

<!-- RAPIDSPEC:START -->
# Archive RapidSpec Change

<command_purpose>
Archive completed RapidSpec changes and merge spec deltas to canonical specs.
Move to timestamped archive, validate strictly, close linked Linear issues.
</command_purpose>

<change_id> #$ARGUMENTS </change_id>

<critical_requirement>
MUST verify all tasks completed before archiving.
MUST validate canonical specs after delta merge.
MUST rollback if validation fails.
CAN auto-close linked Linear issues when found.
</critical_requirement>

## Quick Archive (CLI Option)

<thinking>
For basic archiving without spec delta merging, use CLI.
For full archive with spec delta merging to canonical specs, use this slash command.
</thinking>

**Basic archive with CLI (no spec merging):**
```bash
# Simple timestamp + move to archive
rapid archive <change-id>

# Skip validation check
rapid archive <change-id> --skip-validation
```

**This CLI command only:**
- Checks task completion (warns if incomplete)
- Moves to archive directory

**For full spec delta merging, Linear closing, and validation, continue with Main Tasks below.**

## Main Tasks

### 1. Verify Completion (ALWAYS FIRST)

<thinking>
Before archiving, verify all work is complete and committed.
Check tasks.md, git status, and test results.
Abort if any critical items incomplete.
</thinking>

**Completion checklist:**
   - Check all tasks in tasks.md marked `[x]`
   - Verify code committed to git
   - Ensure tests passing
   - If incomplete, list remaining tasks and abort

### 2. Archive with Timestamp

<thinking>
This preserves history while clearing active changes directory.
</thinking>

**Actions:**
   - Move `rapidspec/changes/<change-id>/` → `rapidspec/changes/archive/<timestamp>-<change-id>/`
   - Example: `20251111123045-prevent-duplicate-smartlinks`

### 3. Merge Spec Deltas

<thinking>
Apply spec deltas from change to canonical specs.
This updates the source of truth with completed changes.
Preserve RapidSpec format and structure.
</thinking>

**Delta merge process:**
   - Read all `specs/<capability>/spec.md` in change folder
   - Apply deltas to canonical specs in `rapidspec/specs/`:
     - **ADDED** → Add requirement to canonical spec
     - **MODIFIED** → Replace existing requirement in canonical
     - **REMOVED** → Delete from canonical spec
     - **RENAMED** → Rename requirement in canonical
   - Preserve requirement format and scenario structure

### 4. Validate Strictly

<thinking>
After merging deltas, validate canonical specs for errors.
Rollback if validation fails to prevent broken specs.
</thinking>

**Validation:**
   ```bash
   rapid validate --strict
   ```
   - Ensure no errors in canonical specs
   - If errors found, rollback and fix
   - Optionally run: `Task code-verifier` for deep validation

### 5. Close Linear Issue (if linked)

<thinking>
If change was linked to Linear issue, auto-close it.
Add archive reference and completion label.
</thinking>

**Linear integration:**
   - Check proposal.md for Linear issue reference
   - If found: Close issue via API
   - Add comment: "Completed via RapidSpec - archived at: <timestamp>-<change-id>"
   - Add label: "completed"

## Examples

Example 1: Complete Archive
```
User: "/rapid:archive prevent-duplicate-smartlinks"

AI checks:
✓ All tasks completed (6/6)
✓ Code committed (3 commits: abc1234, def5678, ghi9012)
✓ Tests passing (5 E2E, 12 unit)

Archiving...
✓ Moved to: archive/20251111123045-prevent-duplicate-smartlinks/

Merging spec deltas...
Reading: specs/smart-links/spec.md
✓ ADDED: Duplicate Prevention requirement
✓ MODIFIED: Smart Link Creation requirement
Updated: rapidspec/specs/smart-links/spec.md

Validating...
Running: rapid validate --strict
✓ No errors found

Linear integration...
✓ Found linked issue: #124 "Add duplicate prevention"
✓ Closed Linear issue #124
✓ Added comment with archive details

Done! Change archived successfully.

Summary:
- Archive: archive/20251111123045-prevent-duplicate-smartlinks/
- Specs updated: smart-links
- Linear closed: #124
```

Example 2: Incomplete Tasks
```
User: "/rapid:archive add-auth-mutex"

AI checks:
✓ Code committed (2 commits)
✓ Tests passing
⚠️  Tasks incomplete (4/6 completed)

Remaining tasks:
- [ ] Task 2.2: Add error handling for mutex timeout
- [ ] Task 3.1: Update docs with new auth flow

Cannot archive: 2 tasks incomplete

Options:
1. Complete remaining tasks first
2. Archive with incomplete tasks (not recommended)

Which? (1 or 2)
```

Example 3: Validation Fails
```
User: "/rapid:archive add-feature"

AI:
✓ All tasks completed
✓ Code committed
✓ Tests passing

Archiving...
✓ Moved to archive/

Merging spec deltas...
✓ Applied deltas

Validating...
❌ Error in rapidspec/specs/auth/spec.md:
  - Line 42: Scenario missing WHEN clause

Rolling back...
✓ Restored from archive

Fix validation errors first:
1. Review: rapidspec/specs/auth/spec.md
2. Add missing WHEN clause
3. Retry: /rapid:archive add-feature
```

**Anti-Patterns**

❌ Don't: Archive with incomplete tasks
```
Bad: 3/6 tasks done → Archive anyway
→ Incomplete implementation in production
```

✅ Do: Complete all tasks first
```
Good: Verify 6/6 tasks → Then archive
```

❌ Don't: Skip validation
```
Bad: Merge deltas → Skip validation → Broken specs
```

✅ Do: Always validate after merge
```
Good: Merge deltas → Validate strictly → Fix errors
```

**Reference**
- Archive path: `rapidspec/changes/archive/<timestamp>-<change-id>/`
- Canonical specs: `rapidspec/specs/<capability>/spec.md`
- Validate command: `rapid validate --strict`
- Linear API: Auto-close issues when found in proposal.md
- User says "yes" (proceed), "wait" (wait) if issues found

<!-- RAPIDSPEC:END -->
