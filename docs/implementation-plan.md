# Implementation Plan

## Milestone 1: Single-agent core

- define the first workflow
- define the success metric
- create task and run schemas
- add persistent run storage
- add a startup retrieval step

## Milestone 2: Memory model

- define memory types
- create memory schema
- separate memory from raw traces
- add tags and source references
- add retrieval query shape

## Milestone 3: Run loop

- implement run start
- implement execution
- append all tool events to the trace
- summarize the run
- generate candidate memory items

## Milestone 4: Retrieval

- implement relevance scoring
- blend relevance, recency, confidence, and utility
- limit the number of loaded memories
- reject expired items
- test retrieval quality

## Milestone 5: Promotion and verification

- implement a judge step
- require trace support for promotion
- reject duplicates
- promote repeatable lessons
- promote repeated failures as cautions

## Milestone 6: Consolidation and observability

- add dedupe and merge jobs
- add expiry cleanup
- add logging for retrievals and promotions
- add metrics for memory health
- add regression tests for memory pollution

## Milestone 7: Multi-agent expansion

- split into planner, executor, memory writer, and judge roles
- keep local memory first
- add a coordinator only after the single-agent loop is stable
- create a shared skill store for validated reusable procedures
- keep inter-agent messages structured

## Build order

1. single-agent loop
2. persistent trace storage
3. retrieval
4. verification and promotion
5. consolidation
6. observability
7. guardrails
8. multi-agent split

