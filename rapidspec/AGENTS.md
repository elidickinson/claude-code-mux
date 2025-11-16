# RapidSpec Instructions

Instructions for AI coding assistants using RapidSpec for spec-driven development.

> RapidSpec: Checkpoint-based spec-driven workflow with AI agents

---

## Quick Start

```bash
# 1. Create proposal with research and verification
/rapid:proposal [feature-name]

# 2. Implement step-by-step with checkpoints
/rapid:apply [change-id]

# 3. Review with specialized agents (optional)
/rapid:review [change-id]

# 4. Update tasks and commit
/rapid:commit

# 5. Archive after deployment
/rapid:archive [change-id]

```

---

## Core Workflow

### `/rapid:proposal` - Create Change Proposal

Creates comprehensive proposals with automatic verification and research.

**What it does:**
1. **Verifies actual code** - Reads files, checks git history (prevents "imaginary code")
2. **Researches best practices** - WebSearch, reference repositories
3. **Presents options** - 2-3 approaches with pros/cons/costs
4. **Waits for decision** - User chooses option ("1", "2", "ㄱㄱ")
5. **Generates files** - Creates proposal.md, tasks.md, spec deltas
6. **Validates strictly** - Ensures correctness before completing

**When to use:**
- New features or capabilities
- Breaking changes
- Architecture changes
- Performance optimizations that change behavior

**Example:**
```
/rapid:proposal add-user-authentication
```

### `/rapid:apply` - Step-by-Step Implementation

Implements proposals with checkpoint-based workflow.

**What it does:**
1. Reads proposal.md, tasks.md, design.md
2. Implements tasks one by one (5-10 min each)
3. Shows diff (Before → After) before each change
4. Pauses at checkpoints for approval
5. Handles direction changes mid-implementation
6. Suggests review when complete

**Checkpoint workflow:**
- "ㄱㄱ" (go) - Continue to next task
- "잠깐" (wait) - Pause and revise current task
- "아니" (no) - Skip or change approach
- User can test at any checkpoint

**Example:**
```
/rapid:apply add-user-authentication
```

### `/rapid:review` - Comprehensive Agent Reviews

Runs quality review agents (optional).

**What it does:**
1. Reads spec and git changes
2. Runs specialized agents:
   - Code verification (prevents imaginary code)
   - Security audit (auth, permissions)
   - Code quality review
   - Architecture patterns
   - Database safety
   - Test coverage

**Review severity:**
- ❌ **Critical**: Must fix before commit
- ⚠️ **Warning**: Should fix (non-blocking)
- ℹ️ **Info**: Nice to have

**When to use:**
- After `/rapid:apply` (optional)
- Before commits
- Can skip for speed

**Example:**
```
/rapid:review add-user-authentication
```

### `/rapid:commit` - Task Update & Commit

Updates tasks.md and creates conventional commit.

**What it does:**
1. Reviews git status and diff
2. Matches changes to tasks
3. Updates tasks.md with completed items
4. Captures discovered work
5. Generates conventional commit message

**Commit format:**
```
feat(scope): brief description

- Task 1.1: Description
- Task 1.2: Description

Discovered work:
- Additional task found

🤖 Generated with RapidSpec
Co-Authored-By: Claude <noreply@anthropic.com>
```

**Example:**
```
/rapid:commit
```

### `/rapid:archive` - Archive Deployed Change

Archives completed changes after deployment.

**What it does:**
1. Moves `changes/[id]/` → `changes/archive/YYYYMMDDhhmmss-[id]/`
2. Updates `specs/` with merged changes
3. Validates archived structure

**Example:**
```
/rapid:archive add-user-authentication
```


## Directory Structure

```
rapidspec/
├── specs/                  # Current truth - what IS built
│   └── [capability]/
│       └── spec.md         # Requirements and scenarios
├── changes/                # Proposals - what SHOULD change
│   ├── [change-name]/
│   │   ├── proposal.md     # Why, what, impact
│   │   ├── tasks.md        # Implementation checklist
│   │   ├── investigation.md # Code analysis (optional)
│   │   ├── research.md     # Best practices (optional)
│   │   └── specs/          # Delta changes
│   │       └── [capability]/
│   │           └── spec.md # ADDED/MODIFIED/REMOVED
│   └── archive/            # Completed changes (timestamped)
```

---

## Three-Stage Workflow

### Stage 1: Creating Changes (Proposal)

**When to create proposal:**
- New features or capabilities
- Breaking changes (API, schema)
- Architecture changes
- Performance optimizations that change behavior

**Skip proposal for:**
- Bug fixes (restore intended behavior)
- Typos, formatting, comments
- Dependency updates (non-breaking)
- Tests for existing behavior

**Workflow:**
1. Run `/rapid:proposal [change-id]`
2. AI verifies actual code (prevents "imaginary code")
3. AI researches best practices
4. AI presents 2-3 options with pros/cons/costs
5. User chooses option ("1", "2", "ㄱㄱ")
6. AI generates and validates files

**Files created:**
- `proposal.md` - Why, what, impact
- `tasks.md` - Step-by-step implementation
- `specs/[capability]/spec.md` - Spec deltas
- `investigation.md` - Code analysis (optional)
- `research.md` - Best practices (optional)

### Stage 2: Implementing Changes (Apply)

**Workflow:**
1. Run `/rapid:apply [change-id]`
2. AI reads spec files
3. AI lists all tasks
4. For each task:
   - Shows current code
   - Shows proposed changes
   - Shows diff (Before → After)
   - Waits for "ㄱㄱ" to proceed
   - Implements
   - Checkpoint: "ㄱㄱ" (continue) or "잠깐" (wait)
5. Suggests `/rapid:review` when complete

**Checkpoint handling:**
- User can test at any checkpoint
- Say "잠깐" (wait) to pause and revise
- Say "아니" (no) to skip or change approach
- AI updates tasks.md to reflect changes

### Stage 3: Archiving Changes (Archive)

**After deployment:**
1. Run `/rapid:archive [change-id]`
2. AI moves to archive with timestamp
3. AI updates specs with merged changes
4. AI validates structure

---

## Proposal File Format

### proposal.md

```markdown
# Change: [Brief Description]

## Why
[1-2 sentences: problem or opportunity]

## Code Verification
- [x] Read actual files: @path/to/file:line
- [x] Git history checked: [findings]
- [x] Existing patterns found: [patterns]

## What Changes

### Before (Verified Actual Code)
\`\`\`
// @src/path/file.ext:42
[actual current code]
\`\`\`

### After (Proposed)
\`\`\`
[new code]
\`\`\`

## Options

### Option 1: [Approach] ⭐ (Recommended)
**Pros:** [benefits]
**Cons:** [drawbacks]
**Cost:** Time: [X], Risk: [low/med/high], Complexity: [low/med/high]

### Option 2: [Alternative]
[same structure]

## Recommendation
Option 1 because: [evidence-based reasoning]

## Impact
- Affected specs: [list]
- Affected files: [list with line numbers]
- Breaking changes: [yes/no, details]
```

### tasks.md

```markdown
## 1. Implementation

### 1.1 [Step Name] (X min) - Checkpoint ⏸
- [ ] [specific task]
- [ ] [specific task]
**Checkpoint:** User can test here

### 1.2 [Next Step] (Y min) - Checkpoint ⏸
- [ ] [specific task]
**Checkpoint:** User can test here

## 2. Validation
- [ ] Agent reviews passed
- [ ] Tests added
- [ ] Documentation updated
```

### specs/[capability]/spec.md (Delta)

```markdown
## ADDED Requirements
### Requirement: [New Feature]
The system SHALL [requirement text]

#### Scenario: [Success case]
- **WHEN** [condition]
- **THEN** [expected result]

#### Scenario: [Error case]
- **WHEN** [error condition]
- **THEN** [error handling]

## MODIFIED Requirements
### Requirement: [Existing Feature]
[Complete modified requirement - paste full text]

#### Scenario: [Updated scenario]
- **WHEN** [condition]
- **THEN** [new expected result]

## REMOVED Requirements
### Requirement: [Deprecated Feature]
**Reason**: [why removing]
**Migration**: [how to migrate]
```

---

## Spec Format Rules

### Critical: Scenario Formatting

**CORRECT** (use #### headers):
```markdown
#### Scenario: User login success
- **WHEN** valid credentials provided
- **THEN** return success response
```

**WRONG** (don't use bullets or bold):
```markdown
- **Scenario: User login**  ❌
**Scenario**: User login     ❌
### Scenario: User login      ❌
```

**Every requirement MUST have at least one scenario.**

### Delta Operations

- `## ADDED Requirements` - New capabilities
- `## MODIFIED Requirements` - Changed behavior (paste full requirement)
- `## REMOVED Requirements` - Deprecated features
- `## RENAMED Requirements` - Name changes only

**When to use ADDED vs MODIFIED:**
- **ADDED:** New capability that can stand alone
- **MODIFIED:** Changes existing requirement behavior
  - **MUST** paste complete requirement from `specs/[capability]/spec.md`
  - Include all scenarios (existing + new)
  - Archiver replaces entire requirement

**RENAMED example:**
```markdown
## RENAMED Requirements
- FROM: `### Requirement: Login`
- TO: `### Requirement: User Authentication`
```

---

## Best Practices

### Simplicity First
- Default to <100 lines of new code
- Single-file implementations until proven insufficient
- Avoid frameworks without clear justification
- Choose proven patterns

### Complexity Triggers
Only add complexity with:
- Performance data showing need
- Concrete scale requirements
- Multiple proven use cases

### Clear References
- Use `file.ext:42` format for code locations
- Reference specs as `specs/capability/spec.md`
- Link related changes and PRs

### Capability Naming
- Use verb-noun: `user-auth`, `payment-capture`
- Single purpose per capability
- 10-minute understandability rule
- Split if description needs "AND"

### Change ID Naming
- Use kebab-case: `add-two-factor-auth`
- Verb-led prefixes: `add-`, `update-`, `remove-`, `refactor-`
- Ensure uniqueness (append `-2`, `-3` if needed)

---

## AI Agents

RapidSpec includes specialized AI agents for code quality, security, and architecture reviews.

**Usage:**
- Installed via plugin: `npx claude-plugins install rapid-spec`
- Available in `agents/` directory
- Use with `@agent-name` syntax (e.g., `@agent-code-reviewer`)
- Automatically invoked during `/rapid:review`
- Can be called manually anytime

**Agent types:**
- Code verification and quality
- Security and permissions
- Architecture patterns
- Database safety
- Test coverage
- Performance analysis

See `agents/` directory for full list and documentation.

---

## User Communication Style

- User may use Korean/English mix
- Common responses: "ㄱㄱ" (go), "잠깐" (wait), "아니" (no)
- Always show diffs (Before → After) for code changes
- Verify actual code (no "imaginary code")
- Wait for user approval at checkpoints

---

## Quick Reference

### Stage Indicators
- `changes/` - Proposed, not yet built
- `specs/` - Built and deployed
- `archive/` - Completed changes

### File Purposes
- `proposal.md` - Why and what
- `tasks.md` - Implementation steps
- `design.md` - Technical decisions (optional)
- `spec.md` - Requirements and behavior

### Command Summary
```bash
/rapid:proposal <name>     # Create proposal
/rapid:apply <name>        # Implement with checkpoints
/rapid:review <name>       # Run agent reviews
/rapid:commit              # Update tasks and commit
/rapid:archive <name>      # Archive after deployment
```

---

Remember: **Verify → Research → Present Options → Wait for Decision → Implement with Checkpoints**
