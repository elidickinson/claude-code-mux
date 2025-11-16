---
name: RapidSpec: Triage
description: Review findings one by one and add selected items to tasks.md
category: RapidSpec
tags: [rapidspec, triage, review]
allowed-tools: Read, Edit, Write
argument-hint: <change-id>
---

<!-- RAPIDSPEC:START -->
# Triage RapidSpec Review Findings

<command_purpose>
Present review findings one by one for triage. Allows individual judgment on which issues to fix now, defer, or skip. Converts accepted findings into actionable tasks in tasks.md.
</command_purpose>

**IMPORTANT: DO NOT CODE ANYTHING DURING TRIAGE!**

This command is for:
- Triaging code review findings from `/rapid:review`
- Converting important findings into tasks
- Prioritizing issues by severity
- Deferring low-priority improvements

<change_id> #$ARGUMENTS </change_id>

<critical_requirement>
MUST present findings one by one, not in batch.
MUST wait for user decision before proceeding to next finding.
MUST update tasks.md when user accepts a finding.
NEVER implement fixes during triage - only track them.
</critical_requirement>

## Main Tasks

### 1. Read Review Context (ALWAYS FIRST)

<thinking>
First, understand what was reviewed and what findings were discovered.
Load the change context to properly categorize findings.
</thinking>

**Immediate Actions:**
- [ ] Read `rapidspec/changes/<change-id>/proposal.md` - Understand scope
- [ ] Check if review findings exist (from previous `/rapid:review` run)
- [ ] Read `rapidspec/changes/<change-id>/tasks.md` - Current task list

### 2. Present Each Finding

<thinking>
Show findings one at a time with full context.
User needs to see severity, impact, and effort to make informed decisions.
Track progress so user knows how many remain.
</thinking>

**Presentation Format:**

```
┌─────────────────────────────────────────────────────────┐
│ Triage Progress: X/Y findings | Est. Z min remaining    │
└─────────────────────────────────────────────────────────┘

---
Finding #N: [Brief Title]

Severity: 🔴 P1 (CRITICAL) / 🟡 P2 (IMPORTANT) / 🔵 P3 (NICE-TO-HAVE)

Category: [Security/Performance/Architecture/Quality/Testing/etc.]

Description:
[Detailed explanation of the issue or improvement]

Location: [file_path:line_number]

Problem:
[What's wrong or could be better]

Impact:
[Why this matters, what could happen if not fixed]

Proposed Solution:
[How to fix it - specific actionable steps]

Estimated Effort: Small (< 30min) / Medium (30min-2h) / Large (> 2h)

---
Add to tasks?
1. yes - add to tasks.md as new task
2. next - skip this finding
3. custom - modify severity/description before adding
```

### 3. Handle User Decision

**When user says "yes":**

1. **Determine task section in tasks.md**
   - Critical (P1) → Add to "## 0. Critical Fixes" section (create if needed)
   - Important (P2) → Add to existing implementation section or create "## X. Code Quality"
   - Nice-to-have (P3) → Add to "## Z. Future Improvements" section

2. **Format as task:**
   ```markdown
   ### X.Y [Task Name from Finding] (Effort: Small/Medium/Large) - Checkpoint ⏸
   - [ ] [Specific action from proposed solution]
   - [ ] [Additional steps if needed]
   - [ ] Test the fix
   **Severity:** 🔴 P1 / 🟡 P2 / 🔵 P3
   **Location:** [file:line from finding]
   **Checkpoint:** Verify fix resolves the issue
   ```

3. **Update tasks.md**
   - Insert into appropriate section
   - Renumber tasks if needed
   - Preserve existing checkpoint structure

4. **Confirm creation:**
   ```
   ✅ Added to tasks.md: Section X.Y - [Task Name]
   ```

**When user says "next":**
- Skip to next finding
- Track skipped items for final summary

**When user says "custom":**
- Ask: "What to modify? (severity/description/solution/effort)"
- Update the finding
- Present revised version
- Ask again: yes/next/custom

### 4. Progress Tracking

<thinking>
Show clear progress so user knows how much remains.
Estimate time based on average triage speed.
</thinking>

**Track and display:**
- Current finding number (X/Y)
- Findings processed
- Estimated time remaining
- Running tally (accepted vs skipped)

**Example progress:**
```
┌─────────────────────────────────────────────────────────┐
│ Triage Progress: 5/12 findings | Est. 7 min remaining   │
│ (3 added to tasks, 2 skipped)                            │
└─────────────────────────────────────────────────────────┘
```

### 5. Final Summary

After all findings processed:

```markdown
╔═══════════════════════════════════════════════════════╗
║         Triage Complete - Y Findings Reviewed         ║
╚═══════════════════════════════════════════════════════╝

Added to tasks.md (X tasks):
  🔴 Critical (P1):
    - Task 0.1: Fix RLS policy on admin_logs
    - Task 0.2: Add transaction boundaries

  🟡 Important (P2):
    - Task 3.1: Extract complex validation logic
    - Task 3.2: Add missing index on release_id

  🔵 Nice-to-have (P3):
    - Task 5.1: Add loading state for better UX

Skipped (Z findings):
  - Finding #2: Variable naming (cosmetic)
  - Finding #8: Micro-optimization (premature)
  - Finding #11: Additional test case (low priority)

Updated: rapidspec/changes/<change-id>/tasks.md

Next Steps:
1. Review updated tasks: cat rapidspec/changes/<change-id>/tasks.md
2. Implement fixes: /rapid:apply <change-id>
3. Or resolve in parallel: /rapid:resolve-parallel <change-id>
```

## Severity Guidelines

**🔴 P1 (CRITICAL) - Must fix before merge:**
- Security vulnerabilities (missing RLS, SQL injection, XSS)
- Data loss risks (missing transactions, race conditions)
- Breaking changes (API errors, crashes)
- Compliance violations (GDPR, accessibility)

**🟡 P2 (IMPORTANT) - Should fix soon:**
- Performance issues (N+1 queries, missing indexes)
- Code quality (high complexity, poor patterns)
- Missing tests (edge cases, error handling)
- Technical debt (deprecated APIs, outdated patterns)

**🔵 P3 (NICE-TO-HAVE) - Fix when time permits:**
- UX improvements (loading states, better messages)
- Code style (naming, formatting)
- Documentation (comments, README)
- Micro-optimizations (minor performance gains)

## Examples

### Example 1: Accept Critical Finding

```
You: "/rapid:triage add-smart-links"

AI:
┌─────────────────────────────────────────────────────────┐
│ Triage Progress: 1/8 findings | Est. 8 min remaining    │
└─────────────────────────────────────────────────────────┘

---
Finding #1: Missing RLS Policy on smart_links Table

Severity: 🔴 P1 (CRITICAL)
Category: Security

Description:
The smart_links table allows any authenticated user to read all links
from all users. No Row Level Security policy is enabled.


Problem:
1. User A creates smart link
2. User B can query and see User A's links
3. Privacy violation - links may contain sensitive URLs

Impact:
- Users can see each other's private links
- Violates user privacy expectations
- Potential data leak of sensitive URLs

Proposed Solution:
1. Enable RLS on smart_links table
2. Add policy: users can only see their own links
3. Add policy: admins can see all links

```sql
ALTER TABLE smart_links ENABLE ROW LEVEL SECURITY;

CREATE POLICY smart_links_user_policy ON smart_links
  FOR SELECT
  USING (auth.uid() = user_id OR has_role('admin'));

CREATE POLICY smart_links_insert_policy ON smart_links
  FOR INSERT
  WITH CHECK (auth.uid() = user_id);
```

Estimated Effort: Small (20 minutes)

---
Add to tasks?
1. yes
2. next
3. custom

You: "yes"

AI:
✅ Added to tasks.md:
   Section 0.1: Enable RLS on smart_links table

Updated tasks.md:
```markdown
## 0. Critical Fixes (Must complete before merge)

### 0.1 Enable RLS on smart_links table (20 min) - Checkpoint ⏸
- [ ] Add ALTER TABLE smart_links ENABLE ROW LEVEL SECURITY
- [ ] Create SELECT policy for user's own links + admin access
- [ ] Create INSERT policy for authenticated users
- [ ] Test: Verify users cannot see each other's links
- [ ] Test: Verify admins can see all links
**Severity:** 🔴 P1
**Checkpoint:** RLS enabled and policies tested
```

┌─────────────────────────────────────────────────────────┐
│ Triage Progress: 2/8 findings | Est. 7 min remaining    │
│ (1 added, 0 skipped)                                     │
└─────────────────────────────────────────────────────────┘

---
Finding #2: Complex validation function
...
```

### Example 2: Skip Low Priority

```
Finding #5: Improve variable naming in validateSmartLink

Severity: 🔵 P3 (NICE-TO-HAVE)
Category: Code Style

Description:
Some variable names could be more descriptive:
- `rl` → `releaseId`
- `lnk` → `link`
- `isDup` → `isDuplicate`

Location: src/lib/validation.ts:42-60

Impact:
Slightly harder to read, but functionality is clear from context.

Proposed Solution:
Rename variables to full descriptive names.

Estimated Effort: Small (10 minutes)

---
Add to tasks?
1. yes
2. next
3. custom

You: "next"

AI:
⏭️ Skipped Finding #5 (code style - low priority)

┌─────────────────────────────────────────────────────────┐
│ Triage Progress: 6/8 findings | Est. 2 min remaining    │
│ (3 added, 3 skipped)                                     │
└─────────────────────────────────────────────────────────┘

---
Finding #6: Missing test for concurrent creation
...
```

### Example 3: Customize Before Adding

```
Finding #4: Add caching for release queries

Severity: 🟡 P2 (IMPORTANT)
Category: Performance

Description:
Release queries are repeated multiple times per request.
Adding caching could reduce database load.

Location: src/app/api/releases/route.ts:23

Impact:
Moderate performance improvement, especially under load.

Proposed Solution:
Add Redis caching with 5-minute TTL.

Estimated Effort: Medium (1-2 hours)

---
Add to tasks?
1. yes
2. next
3. custom

You: "custom"

AI: "What to modify? (severity/description/solution/effort)"

You: "severity - this is critical for our performance goals, make it P1"

AI:
Updated Finding #4:

Severity: 🔴 P1 (CRITICAL) ← Changed from P2
Category: Performance

[rest stays same]

---
Add to tasks?
1. yes
2. next
3. custom

You: "yes"

AI: ✅ Added to tasks.md: Section 0.3 - Add release query caching
```

## Anti-Patterns

❌ **Don't: Batch accept without review**
```
Bad: "yes to all" → No individual judgment
```

✅ **Do: Review each finding individually**
```
Good: Consider each finding's context, priority, and effort
```

❌ **Don't: Implement during triage**
```
Bad: User says "yes" → AI starts coding
→ Triage should only track, not implement
```

✅ **Do: Only update tasks.md**
```
Good: User says "yes" → Add to tasks.md → Continue triage
```

❌ **Don't: Skip critical findings**
```
Bad: P1 Security issue → "next"
→ Critical issues should always be addressed
```

✅ **Do: At least track critical items**
```
Good: If not fixing now, note why and when to fix
```

## Reference

- Triage is the bridge between `/rapid:review` and `/rapid:apply`
- Review finds issues → Triage decides priority → Apply fixes them
- Track all accepted items in tasks.md for systematic resolution
- Use `/rapid:resolve-parallel` to fix multiple tasks efficiently
- Can re-run triage if new findings discovered during implementation

<!-- RAPIDSPEC:END -->
