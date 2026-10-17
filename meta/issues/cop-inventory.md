# List the COP services the underworld scripts use

## Summary

Scripts call engine services with `COP nn`. The runtime models some, steps
over others by their operand length, and stops an actor at a service whose
length it cannot get. There is no list of which services the underworld
scripts use, so the open work is not countable.

## Dependencies

- [Finish the partial COP services](partial-cop-services.md)

## Requirements

- One function in the runtime tells what it does with each service: models
  it, steps over it, or stops. The dispatch uses the same function.
- A static walk from the underworld maps' spawn lists finds each `COP` site:
  it follows the script branches, the spawned scripts and the native code
  between the services, and records where it cannot go on.
- A generated doc lists each service with its site count and the runtime's
  handling, and the places where the walk stopped.

## Acceptance Criteria

- Unit tests cover the walk on synthetic images and the handling function.
- A local test fails when the generated doc is not current.
- Workspace checks pass.

## Notes

- The idea came from Phase Distorter's list of script engine calls
  (`cpp/docs/native-action-services.tsv`).

## Progress

- Done: `actors::route`/`handling`, `service_sites` and `docs/cop-inventory.md`
  (109 services at 1748 sites; 7 stepped over: `3C`, `5F`, `64`, `AA`, `AF`,
  `E4`, `E5`; no walk stops).
- The native derivation misreads `COP 46` (3 operand bytes, where every site
  has 7) and finds more paths for `DD`, `DE` and `EE` than the runtime
  models; `the_fixed_layouts_agree_with_the_native_handlers` lists them.
