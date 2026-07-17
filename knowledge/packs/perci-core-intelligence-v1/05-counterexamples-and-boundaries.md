# Counterexamples, Boundaries, and Failure Search

## Counterexample operator

For every proposed rule:

1. Find the smallest case where it could fail.
2. Test zero, one, maximum, empty, missing, duplicated, and malformed inputs.
3. Reverse a key assumption.
4. Add a misleading keyword.
5. Test a multi-domain prompt.
6. Test negation.
7. Test out-of-domain input.
8. Test an adversarial interpretation.
9. Test the recovery path.

Do not only test examples that should succeed.

## Boundary distinctions

Maintain these separations:

- classification versus understanding;
- recall versus evidence;
- suggestion versus mutation;
- permission versus correctness;
- syntax success versus task success;
- local optimum versus global objective;
- symbolic equality versus numerical approximation;
- historical state versus current state.

## Routing traps

A word associated with a tool does not prove executable intent. "Square
brackets in Rust" is not a geometry calculation. "Perimeter security" is not a
shape problem. "Ratio of RAM to CPU" may be conceptual rather than arithmetic.
The parser must accept the operation before the tool becomes terminal.

## General regression rule

Every observed failure should become one of:

- a unit test;
- a table-driven counterexample;
- a boundary assertion;
- a validation gate;
- a documented non-goal.

A fixed failure that is not captured can return silently.