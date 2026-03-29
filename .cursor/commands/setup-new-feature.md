# Setup New Feature

## Overview

Systematically set up a new feature from initial planning through to implementation structure. Walk through each step interactively with the user before moving on.

## Steps

### 1. Define requirements

- **Clarify scope and goals:** Ask the user what the feature does, who it's for, and what problem it solves. Summarize back in 2–3 sentences for confirmation.
- **Identify user stories and acceptance criteria:** Draft concrete user stories (`As a <user>, I want <goal> so that <benefit>`) and testable acceptance criteria. Present them to the user for review.
- **Plan technical approach:** Based on the requirements, outline which parts of the codebase are affected (backend, frontend, database, config). Call out risks, unknowns, or decisions that need input.

### 2. Create feature branch

- **Branch from the correct base:** Determine the right base branch (usually `main` or `develop`). Create a feature branch with a descriptive name following the convention `feature/<short-slug>`.
- **Set up local development environment:** Verify the dev server runs cleanly. Install or configure any new dependencies the feature requires.
- **Configure new dependencies:** If new packages or Cargo crates are needed, add them now and confirm they build/install successfully.

### 3. Plan architecture

- **Design data models and APIs:** If the feature involves new data, sketch out the schema changes (DB tables, Rust structs, TypeScript types). If it involves new Tauri commands or API endpoints, define their signatures.
- **Plan UI components and flow:** List the Svelte components that need to be created or modified. Describe the user flow (screens, interactions, state transitions).
- **Consider testing strategy:** Identify what should be tested (unit, integration, e2e) and note any edge cases worth covering.

## Checklist

Before moving to implementation, confirm each item with the user:

- [ ] Requirements documented (scope, goals, constraints)
- [ ] User stories written with acceptance criteria
- [ ] Technical approach planned and agreed upon
- [ ] Feature branch created and pushed
- [ ] Development environment ready (builds, runs, dependencies installed)
- [ ] Data models / API signatures defined
- [ ] UI component plan outlined
- [ ] Testing strategy noted

## Behavior

- Work through the steps **in order**. Do not skip ahead.
- After each step, present the output to the user and wait for confirmation before proceeding.
- Use the todo list tool to track progress through the checklist.
- If the user provides a feature name/description upfront, use it to pre-fill as much as possible, but still confirm each section.
- Keep outputs concise — bullet points and short paragraphs, not essays.
