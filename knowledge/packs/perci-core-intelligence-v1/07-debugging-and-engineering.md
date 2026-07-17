# Debugging and Engineering Protocol

## Debugging

1. Capture the exact error.
2. Identify the process and file that produced it.
3. Trace the controlling call path.
4. Reproduce the smallest failing case.
5. Inspect actual inputs, environment, and state.
6. Form multiple candidate causes.
7. Run a discriminating test.
8. Patch the cause, not the symptom.
9. Add a regression test.
10. Compile in the target configuration.
11. Run integration and operational checks.

## Minimal coherent change

Change the smallest set of components that restores the violated invariant.
Expand only when evidence shows the defect is architectural.

## Code-generation preflight

Identify language version, operating system, dependency policy, input/output
contract, failure semantics, compatibility constraints, security boundary, and
rollback.

## Implementation rules

- validate boundaries;
- handle errors explicitly;
- avoid hidden mutable state;
- preserve idempotence where practical;
- use checked arithmetic;
- bound memory and output;
- separate parsing from execution;
- keep durable mutation explicit;
- benchmark release builds rather than inferring speed from source.

## Review questions

- What invariant does this code enforce?
- What happens on empty, malformed, oversized, or hostile input?
- What resource is unbounded?
- What happens if a child process hangs or exits early?
- Is the fallback safer than the primary path?
- Does the test exercise the real failure?
- Can the change be rolled back cleanly?