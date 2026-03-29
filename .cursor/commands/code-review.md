# Code Review

## Overview

Perform a thorough code review that verifies functionality, maintainability, and security before approving a change. Focus on architecture, readability, performance implications, and provide actionable suggestions for improvement.

## Steps

### 1. Understand the change

- **Read context:** Read the PR description, linked issues, or the user's explanation of the change. If a GitHub PR URL is provided, fetch it with `gh`.
- **Identify scope:** List all files and features impacted. Use git diff (staged, unstaged, or between branches) to see exactly what changed.
- **Note questions:** Record any assumptions or ambiguities to clarify with the author before passing judgment.

### 2. Validate functionality

- **Confirm intended behavior:** Trace through the code paths to verify the change delivers what it claims.
- **Exercise edge cases:** Mentally (or locally) walk through boundary conditions, empty inputs, null/undefined values, concurrent access, and error triggers.
- **Check error handling:** Ensure errors are caught at the right level, messages are informative, and logging is present where it aids debugging.

### 3. Assess quality

- **Readability:** Functions should be focused and well-named. Control flow should be easy to follow. Complex logic should have explanatory comments.
- **Duplication and dead code:** Flag copy-pasted blocks that should be extracted, and unreachable or unused code that should be removed.
- **Tests and documentation:** Verify that new behavior has corresponding tests, and that comments/docs reflect the latest changes — not stale descriptions.

### 4. Review security and risk

- **Injection and validation:** Look for SQL injection, XSS, command injection, path traversal, or missing input validation.
- **Secrets and credentials:** Confirm no API keys, tokens, or passwords are hardcoded or logged.
- **Performance and scalability:** Flag O(n²) loops, unbounded queries, missing pagination, large allocations, or blocking calls on the main thread.

## Review Checklist

Work through each item and report its status (pass / concern / fail):

### Functionality

- [ ] Intended behavior works and matches requirements
- [ ] Edge cases handled gracefully
- [ ] Error handling is appropriate and informative

### Code Quality

- [ ] Code structure is clear and maintainable
- [ ] No unnecessary duplication or dead code
- [ ] Tests and documentation updated as needed

### Security & Safety

- [ ] No obvious security vulnerabilities introduced
- [ ] Inputs validated and outputs sanitized
- [ ] Sensitive data handled correctly

### Performance & Architecture

- [ ] No performance regressions or bottlenecks introduced
- [ ] Architecture and design decisions are sound
- [ ] Resource management is correct (memory, file handles, connections)

## Output format

Present the review as:

1. **Summary** — One paragraph describing what the change does and the overall verdict (approve / request changes / needs discussion).
2. **Checklist results** — The checklist above with each item marked and a short note if there's a concern.
3. **Findings** — A numbered list of specific issues or suggestions, each with:
   - **File and location** (file path + line range or function name)
   - **Severity** (critical / warning / nit)
   - **Description** of the problem
   - **Suggestion** with a concrete code example or alternative approach
4. **Positives** — Call out things done well (good patterns, clean abstractions, thorough tests).

## Behavior

- If the user provides a PR URL, fetch the diff and description via `gh` before starting.
- If no PR is given, use `git diff` on the current branch vs its base (usually `main`) to determine what changed.
- Read every changed file before commenting on it — never guess at code you haven't seen.
- Be constructive. Pair every criticism with an actionable suggestion or concrete example.
- Distinguish severity clearly: **critical** = must fix before merge, **warning** = should fix, **nit** = optional improvement.
- Keep feedback concise — no essays, just clear points with code references.
