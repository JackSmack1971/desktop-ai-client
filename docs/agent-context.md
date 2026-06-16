# Agent Context

## Purpose

This repository is being set up to support coding agents that work on a long-running, memory-aware system.

The core idea is simple:
- keep the agent focused on one task at a time
- persist only the useful lessons
- verify before promoting memory
- expand into multiple agents only when the single-agent loop is stable

## Current direction

The current design direction is a memory-first agent system with:
- working memory for the active task
- episodic memory for run summaries and outcomes
- procedural memory for reusable methods
- caution memory for repeated failure modes

## Vocabulary

- `working memory` - what the agent needs for the current run
- `episodic memory` - a compact record of what happened in a prior run
- `procedural memory` - a reusable workflow or method
- `caution memory` - a warning about a known failure pattern
- `promotion` - moving a candidate memory into durable storage
- `consolidation` - deduping, merging, and pruning memory
- `retrieval` - loading the most relevant memory into the next run

## What agents should optimize for

- fewer repeated mistakes
- shorter recovery time after failures
- better reuse of proven workflows
- less context bloat
- stable behavior across sessions

## What agents should avoid

- treating raw chat as the source of truth
- storing every observation as durable memory
- over-retrieving stale or low-value memories
- widening the system into multiple roles too early
- changing the memory format without a migration plan

