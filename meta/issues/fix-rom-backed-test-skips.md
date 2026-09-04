# Make ROM-backed scenario tests skip on clean checkouts

## Summary

Ensure the existing JP and EU child-process scenario tests return successfully before spawning when their required local ROM is absent.

## Dependencies

- [Establish ROM-free continuous integration](rom-free-ci.md)

## Requirements

- Keep ROM-backed assertions unchanged when local dumps exist.
- Avoid requiring child output markers when a test is skipped.

## Acceptance Criteria

- JP and EU scenario tests pass without a `local/` directory.
- The same tests still run their full child scenarios when local dumps exist.

## Verification

- Both integration-test binaries pass and report explicit skips with `local/`
  temporarily absent.
- The full JP and EU ROM-backed scenarios pass with `local/` restored.
