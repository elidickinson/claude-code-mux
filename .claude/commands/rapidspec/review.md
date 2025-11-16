---
name: RapidSpec: Review
description: Comprehensive spec review with security, architecture, and quality agents
category: RapidSpec
tags: [rapidspec, review, quality]
allowed-tools: Read, Bash, Grep, Glob, Task
argument-hint: <change-id>
---

<!-- RAPIDSPEC:START -->
# Review RapidSpec Implementation

<command_purpose>
Perform exhaustive code reviews using multi-agent analysis for deep quality assurance.
Reviews validate implementation against spec, ensuring security, performance, and maintainability.
</command_purpose>

## Introduction

<role>Senior Code Review Architect with expertise in security, performance, architecture, and quality assurance</role>

## Review Target

<review_target> #$ARGUMENTS </review_target>

## Main Tasks

### 1. Read Spec and Implementation (ALWAYS FIRST)

<thinking>
First, I must understand what was planned and what was actually implemented.
Read the RapidSpec proposal and tasks, then compare against actual git changes.
This provides context for all subsequent agent reviews.
</thinking>

<critical_requirement>
MUST read spec files BEFORE running agents. No exceptions.
Agents need context to provide relevant feedback.
</critical_requirement>

**Immediate Actions:**

- [ ] Read `rapidspec/changes/<change-id>/proposal.md` - What was planned
- [ ] Read `rapidspec/changes/<change-id>/tasks.md` - What tasks were defined
- [ ] Run `git diff` - What was actually implemented
- [ ] Compare implementation against proposed changes

### 2. Detect Project Type

<thinking>
Determine the project stack to select appropriate language-specific reviewers.
This informs which agents to run for maximum relevance.
</thinking>

<project_type_detection>

Check for these indicators:

**Next.js Project**:
- `package.json` with `"next"` dependency
- `app/` or `pages/` directory
- `next.config.js`
- `.tsx` or `.ts` files

- `.sql` migration files

**TypeScript Project**:
- `tsconfig.json`
- `.ts` or `.tsx` files

Based on detection, include project-specific reviewers in parallel execution.

</project_type_detection>

### 3. Selective Agent Review

<thinking>
Run core agents always, and additional agents based on what changed.
This provides comprehensive coverage without unnecessary overhead.
Detect what changed (DB, UI, API) and run relevant specialized agents in parallel.
</thinking>

<parallel_tasks>

**Core Agents (always run in parallel)**:
1. Task code-verifier(change_files, proposal_context)
2. Task security-auditor(change_files, migration_files)

**Conditional Agents (run based on changes)**:

Detect changes and run relevant agents in parallel:

```bash
# Check what changed
HAS_COMPONENT_CHANGES=$(git diff --name-only | grep -E "\.tsx|\.jsx|components/")
HAS_TEST_CHANGES=$(git diff --name-only | grep -E "\.test\.|\.spec\.|e2e/")
HAS_API_CHANGES=$(git diff --name-only | grep -E "api/|app/.*route\.ts")
```

Then run in parallel based on detection:

**If database changes** (`HAS_DB_CHANGES`):
- Task data-integrity-guardian(migration_files)
- [database-architect moved to /rapid:proposal Design Review and /rapid:apply Architecture Validation]

**If component changes** (`HAS_COMPONENT_CHANGES`):
- Task code-reviewer(component_files)
- [nextjs-architecture-expert moved to /rapid:proposal Design Review and /rapid:apply Architecture Validation]

**If test changes or new features** (`HAS_TEST_CHANGES` or user-facing):
- Task test-automator(test_files, change_files)

**If API changes** (`HAS_API_CHANGES`):
- Task performance-oracle(api_files)

**Optional (run if explicitly requested)**:
- Task git-history-analyzer(change_files) - Historical context
- Task pr-comment-resolver(change_files) - If resolving PR comments

</parallel_tasks>

**Agent Responsibilities:**

**@agent-code-verifier**: Prevents "imaginary code"
- ✓ Files read before modification
- ✓ Patterns verified with Grep
- ✓ Git history checked
- ✓ Diffs shown for changes

**@agent-security-auditor**: Security & RLS compliance
- ✓ Policies use `has_role()` function
- ✓ Input validation present
- ✓ Auth token handling secure
- ✓ No SQL injection vulnerabilities
- ✓ OWASP Top 10 compliance

**@agent-code-reviewer**: Code quality
- Type safety (no unjustified `any`)
- React/Next.js best practices
- Performance (no O(n²), proper pagination)
- Error handling present
- Code complexity (cognitive <10)
- Testing coverage

**@agent-data-integrity-guardian**: Data integrity & safety
- Migration execution safety (non-blocking)
- Data consistency checks
- Constraint validation
- Index coverage for queries
- No data loss risks

**Note:** Architecture agents (@agent-nextjs-architecture-expert, @agent-database-architect)
now run during /rapid:proposal (Design Review) and /rapid:apply (Architecture Validation)
to catch design issues before implementation starts.

**@agent-test-automator**: Test coverage
- E2E tests present (Playwright)
- Unit tests present (Vitest)
- Edge cases covered
- Coverage target 80%+

**@agent-performance-oracle**: Performance checks
- No O(n²) algorithms
- Proper pagination
- Bundle size impact
- Render performance
- Database query optimization

**@agent-architecture-strategist**: Architecture alignment
- Component structure
- Data flow patterns
- Separation of concerns
- Scalability considerations

**@agent-git-history-analyzer**: Historical context
- Consistency with past patterns
- Breaking change awareness
- Contributor expertise

### 4. Synthesize Findings

<thinking>
Analyze all agent reports to identify patterns and conflicts.
Prioritize by severity: Critical blocks commit, Warnings should be fixed, Info is optional.
Provide actionable fix suggestions with code examples.
</thinking>

<ultrathink_instruction>
Spend maximum cognitive effort synthesizing agent findings.
Look for:
- Common themes across multiple agents
- Conflicting recommendations (resolve based on project context)
- Critical issues that must block commit
- Quick wins that provide high value
</ultrathink_instruction>

**Synthesis Process:**

1. **Group by Severity**
   - ❌ Critical: Security, type errors, broken functionality
   - ⚠️ Warning: Complexity, missing tests, performance
   - 💡 Info: Refactoring, micro-optimizations

2. **Identify Patterns**
   - Multiple agents flagging same issue = high priority
   - Isolated findings = lower priority

3. **Provide Fixes**
   - Include code examples for each issue
   - Link to relevant documentation
   - Estimate fix time

### 5. Generate Report

<thinking>
Present findings in a clear, actionable format.
Show per-agent status, grouped issues, and overall verdict.
Offer to fix warnings automatically.
</thinking>

**Report Format:**

```
┌─────────────────────────────────────────┐
│ 1. Code Verification          ✅ PASS   │
├─────────────────────────────────────────┤
│ ✓ All files read before modification   │
│ ✓ No imaginary code detected           │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│ 2. Security Audit        ⚠️ PASS (1 warn)│
├─────────────────────────────────────────┤
│ ⚠️ [Warning] Missing index on FK        │
│    Table: smart_links                   │
│    Column: release_id                   │
│    Fix:                                 │
│      CREATE INDEX CONCURRENTLY          │
│      idx_smart_links_release_id         │
│      ON smart_links(release_id);        │
└─────────────────────────────────────────┘

╔═════════════════════════════════════════╗
║ OVERALL: ✅ PASSED (1 warning)          ║
╚═════════════════════════════════════════╝

Summary:
- ✅ Critical checks: All passed
- ⚠️ Warnings: 1 (database optimization)

Fix warnings? (yes to fix, skip to ignore)
```

### 6. Offer to Fix Warnings

<thinking>
If warnings found, offer to fix them automatically.
User can say "yes" to proceed with fixes, or "skip" to ignore.
Critical issues MUST be fixed before commit.
</thinking>

**User responses:**
- "yes" or "fix" → Implement fixes automatically
- "skip" or "ignore" → Track warnings but don't block
- User may say "wait" (wait) to review warnings first

## Severity Levels

**❌ Critical (Blocks commit)**
- Security issues (missing RLS, SQL injection)
- Type errors without justification
- Broken functionality (API errors, crashes)
- Async APIs not awaited (Next.js 16)
- Migration without rollback
- No authentication on protected routes

**⚠️ Warning (Should fix)**
- High complexity functions (cognitive >10)
- Missing indexes on foreign keys
- Low test coverage (<80%)
- Unnecessary Client Components
- Missing error handling
- Performance issues (O(n²))

**💡 Info (Nice to have)**
- Refactoring opportunities
- Performance micro-optimizations
- Additional comments
- Better variable names

## Examples

### Example 1: Clean Pass (Component Changes)

```
User: "/rapid:review add-loading-state"

AI thinking:
→ Read proposal: Suspense boundary approach
→ Read tasks: 3 tasks defined
→ Git diff: 3 files changed (all .tsx components)
→ Detect: Next.js + TypeScript project, component changes only

AI detects changes:
✓ HAS_COMPONENT_CHANGES: Yes (3 .tsx files)
✓ HAS_DB_CHANGES: No
✓ HAS_API_CHANGES: No

AI executes in parallel:
→ Task code-verifier → No issues (core, always run)
→ Task security-auditor → No security changes (core, always run)
→ Task code-reviewer → Clean code (component changes detected)
→ Task nextjs-architecture-expert → Proper Suspense usage (Next.js detected)

AI synthesizes:
✓ 4 agents run (2 core + 2 conditional)
✓ All passed
✓ Implementation matches proposal

╔═════════════════════════════════════════╗
║ OVERALL: ✅ PASSED                      ║
╚═════════════════════════════════════════╝

Ready to commit: /rapid:commit add-loading-state
```

### Example 2: Warnings Found (DB + Component Changes)

```
User: "/rapid:review prevent-duplicate-smartlinks"

AI detects changes:
✓ HAS_DB_CHANGES: Yes (1 migration file)
✓ HAS_COMPONENT_CHANGES: Yes (2 .tsx files)

AI executes in parallel:
→ Task code-verifier → Verified all files (core)
→ Task security-auditor → RLS enabled (core)
→ Task database-architect → Found 1 recommendation (DB changes detected)
→ Task code-reviewer → Found 2 warnings (component changes detected)

AI synthesizes:
⚠️ 3 warnings found (non-blocking)
✓ 4 agents run (2 core + 2 conditional)

┌─────────────────────────────────────────┐
│ 3. Code Quality        ⚠️ PASS (2 warn) │
├─────────────────────────────────────────┤
│ ⚠️ [Warning] Complex function           │
│    src/lib/validation.ts:42             │
│    Cognitive complexity: 15 (max 10)    │
│    Suggestion:                          │
│      Extract validation rules           │
│                                         │
│ ⚠️ [Warning] Missing error boundary     │
│    src/app/smart-links/page.tsx         │
│    Suggestion:                          │
│      Wrap in <ErrorBoundary>            │
└─────────────────────────────────────────┘

╔═════════════════════════════════════════╗
║ OVERALL: ✅ PASSED (3 warnings)         ║
╚═════════════════════════════════════════╝

Fix warnings? (yes to fix, skip to ignore)
```

### Example 3: Critical Issues (DB + Auth Changes)

```
User: "/rapid:review add-admin-panel"

AI detects changes:
✓ HAS_DB_CHANGES: Yes (admin_logs table migration)
✓ HAS_COMPONENT_CHANGES: Yes (admin panel pages)

AI executes in parallel:
→ Task code-verifier → Verified (core)
→ Task security-auditor → 🚨 CRITICAL: Missing RLS (core)
→ Task database-architect → Missing rollback (DB changes detected)
→ Task code-reviewer → Admin check incorrect (component changes detected)

AI synthesizes:
❌ 2 critical issues (blocks commit)
✓ 4 agents run (2 core + 2 conditional)

┌─────────────────────────────────────────┐
│ 2. Security Audit        ❌ FAIL        │
├─────────────────────────────────────────┤
│ ❌ [Critical] Missing RLS policy        │
│    Table: admin_logs                    │
│    Security risk: Any user can access   │
│    Fix required:                        │
│      ALTER TABLE admin_logs             │
│        ENABLE ROW LEVEL SECURITY;       │
│      CREATE POLICY admin_logs_policy    │
│        ON admin_logs                    │
│        USING (has_role('admin'));       │
│                                         │
│ ❌ [Critical] Admin check incorrect     │
│    src/app/admin/page.tsx:23            │
│    Current: user?.role === 'admin'      │
│    Problem: Client-side check only      │
│    Fix:                                 │
│      Use has_role('admin') in RLS       │
└─────────────────────────────────────────┘

╔═════════════════════════════════════════╗
║ OVERALL: ❌ FAILED (2 critical)         ║
╚═════════════════════════════════════════╝

Cannot commit until critical issues fixed.

Fix now? (yes to fix)
```

## When to Run

**Recommended:**
- After `/rapid:apply` completes all tasks
- Before `/rapid:commit` to catch issues early
- Any time: "review this" or "check quality"
- After major changes

**Can skip:**
- Minor changes (typo, doc update)
- Confident in implementation
- Tight iteration loop

## Workflow Integration

**Standard flow:**
```
/rapid:apply add-feature
→ /rapid:review add-feature
→ Fix warnings (if any)
→ /rapid:commit add-feature
```

**Quick iteration:**
```
/rapid:apply add-feature
→ /rapid:commit add-feature  (skip review)
```

**Mid-implementation check:**
```
/rapid:apply add-feature (Tasks 1-2)
→ "review this" (check if on track)
→ Continue with remaining tasks
```

## Anti-Patterns

❌ **Don't: Review without reading spec**
```
Bad: Run agents → No context → Irrelevant feedback
```

✅ **Do: Read spec first, then review**
```
Good: Read proposal.md → Understand goal → Run agents → Relevant feedback
```

❌ **Don't: Ignore critical issues**
```
Bad: "skip" critical security issues
→ Compromised system
```

✅ **Do: Always fix critical issues**
```
Good: yes to fix → Resolve all critical → Then commit
```

## Reference

- Review is optional but recommended
- Always run after major changes
- Critical issues block commits
- Warnings tracked but don't block
- User can choose to fix warnings, skip them, or wait to review first
- After review, suggest `/rapid:commit` or fix issues
- Parallel agents = faster reviews (3-6 agents based on changes)

## Notes

- **Core agents always run** (2): code-verifier, security-auditor
- **Conditional agents run based on changes** (2-6 more): Detect what changed and run relevant reviewers
- **Parallel execution** for speed: All selected agents run simultaneously
- **Synthesize findings** across all agents to identify patterns
- **Provide actionable fixes** with code examples
- **Typical review runs 3-6 agents** (not all 11), keeping it fast and relevant

<!-- RAPIDSPEC:END -->
