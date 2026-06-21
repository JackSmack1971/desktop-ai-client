# Surgical Minimalism

## Objective

Implement the smallest complete change that satisfies the explicit request,
preserves existing behavior, and conforms to the repository's established
architecture.

Minimize total system complexity, not merely line count.

## Mandatory Pre-Implementation Ladder

Before adding or changing code, evaluate these options in order and stop at the
first one that completely satisfies the requirement:

1. No change
   - Determine whether the requested behavior already exists.
   - Determine whether documentation, configuration, or correct usage resolves
     the request without changing production code.
   - Do not implement hypothetical requirements.

2. Delete or simplify
   - Prefer removing obsolete logic over adding compensating logic.
   - Prefer correcting an existing implementation over creating a parallel one.
   - Remove dead indirection made unnecessary by the change.

3. Existing repository capability
   - Search the repository for an existing helper, component, command, service,
     type, configuration option, or established pattern.
   - Extend the existing implementation when it has the same responsibility.
   - Do not create a competing implementation.

4. Language or platform capability
   - Prefer the standard library, browser platform, runtime, framework, operating
     system, database, or build tool when it already provides the behavior.
   - Do not wrap a native capability merely to rename it.

5. Installed dependency
   - Reuse an already-installed dependency when it directly and safely solves the
     requirement.
   - Do not recreate a capability already owned by the dependency.

6. Local implementation
   - Only after the preceding options fail, write the minimum implementation
     necessary for the current acceptance criteria.
   - Optimize for directness, readability, and reversibility.

## Scope Constraints

Do not:

- Add features that were not requested.
- Refactor unrelated code.
- Introduce speculative extensibility.
- Design for imagined future consumers.
- Add compatibility layers without an identified compatibility requirement.
- Add fallback behavior without a defined failure mode.
- Create new files when an existing owned file is the correct location.
- Create helpers used only once unless they isolate a meaningful boundary.
- Create interfaces with only one implementation unless a real contract boundary
  requires one.
- Introduce wrappers that merely forward arguments.
- Add factories, registries, adapters, managers, services, or repositories solely
  to make the design appear more architectural.
- Add a dependency when the repository, standard library, or platform can solve
  the problem adequately.
- Replace working code solely because another style is preferred.
- Touch files outside the smallest coherent change surface.
- Add comments that restate obvious code.
- Add tests, fixtures, mocks, or abstractions for behavior outside the requested
  change.

## Abstraction Admission Test

A new abstraction is allowed only when at least one condition is true:

- The current task introduces repeated behavior with the same semantics.
- The abstraction represents a real domain concept.
- It isolates an external or unstable boundary.
- It enforces an architectural invariant.
- It materially reduces current complexity rather than relocating it.
- The repository already uses that abstraction pattern for the same responsibility.
- The user explicitly requested it.

Before introducing the abstraction, be able to name:

- The concrete duplication or boundary it owns.
- The current callers that need it.
- The behavior it prevents from diverging.
- Why a direct implementation would be worse now, not hypothetically later.

If these cannot be named, do not introduce the abstraction.

## Dependency Admission Test

Before adding a dependency, confirm all of the following:

- No existing dependency provides the required behavior.
- The standard library or native platform is insufficient.
- A small local implementation would be less reliable or maintainable.
- The dependency is actively maintained and compatible with the project.
- Its runtime, security, licensing, bundle-size, and operational costs are
  acceptable.
- The requirement is important enough to justify permanent supply-chain surface.

Do not add a dependency merely to save a few straightforward lines.

## Change-Surface Budget

Prefer, in order:

1. No changed files.
2. One existing file.
3. The smallest set of existing files that forms a complete change.
4. A new file only when it has a distinct, durable responsibility.

File count is not an absolute limit. Cross-cutting correctness changes may require
multiple files. Never compress a legitimate architectural boundary merely to
reduce the count.

## Non-Negotiable Correctness Floor

Minimalism must never remove or weaken:

- Trust-boundary input validation.
- Authorization or authentication checks.
- Secret handling.
- Data-integrity protections.
- Error handling needed to prevent corruption, silent failure, or data loss.
- Transactional guarantees.
- Concurrency and cancellation correctness.
- Resource cleanup.
- Accessibility.
- Required auditability or observability.
- Public API compatibility unless the task explicitly changes the contract.
- Repository-defined architectural invariants.
- Explicit user requirements.

Small code is not successful code when it is incomplete, unsafe, or ambiguous.

## Verification

Use the smallest verification that can detect failure of the changed behavior.

- Prefer existing test infrastructure.
- Run focused tests before broad suites.
- Add a test when non-trivial behavior changes and no existing test covers it.
- Do not introduce a new test framework for one change.
- Test externally observable behavior rather than incidental implementation
  structure.
- Do not claim success without executing the relevant available checks.
- If verification cannot be executed, state exactly what remains unverified.

Trivial declarative or one-line changes do not require artificial tests when the
existing compiler, type checker, linter, or build already verifies them.

## Required Working Method

When implementing a change:

1. Identify the explicit acceptance criteria.
2. Inspect the relevant existing implementation.
3. Search for reusable repository capabilities.
4. Evaluate the pre-implementation ladder.
5. State internally why the selected approach is the smallest complete solution.
6. Make the narrowest coherent change.
7. Review the diff for accidental scope expansion.
8. Remove unnecessary additions.
9. Run the smallest meaningful verification.
10. Report the result without presenting rejected complexity as optional extras.

## Diff Review Questions

Before completion, inspect the final diff and ask:

- Can any added file be avoided?
- Can any added abstraction be removed?
- Can any new dependency be avoided?
- Did the change duplicate existing behavior?
- Did unrelated cleanup enter the diff?
- Is every changed line required by the acceptance criteria or correctness floor?
- Could configuration or deletion replace part of the implementation?
- Did simplification weaken security, validation, error handling, accessibility,
  or data integrity?
- Is the resulting code easier to remove or change later?

Revise the diff when any unnecessary complexity remains.

## Completion Report

Keep the final report brief and include:

- What changed.
- What existing or native capability was reused.
- What unnecessary complexity was avoided or removed.
- Which verification commands ran and their result.
- Any remaining limitation that is real and current.

Do not recommend speculative follow-up work.