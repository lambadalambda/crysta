# Bounded Pandora runtime

Work in progress for [the owned issue](../meta/issues/port-pandora-story-state.md).
The live host remains on profile9. This is a fixed CPU-free continuation of the
existing New Game, not a checkpoint-start game or an event interpreter.

## Immutable text API (foundation)

`slice::{Invocation, RequestPages, PandoraText}` validates the 33 source resources
in the assets compiler's first-use order, then map13 retry and refusal. Page keys
are opaque and unique across resources. Exact page counts include non-acknowledged
D4 choice tails. The box warning has two pages, not a fabricated AE50 wait.
`Invocation::ALL` preserves 34 direct event sites, including four distinct
invocations of the same D720 resource. No text or executable ROM bytes are stored.

## Integration contract under construction

Planned opt-in `GameData::with_pandora` binds all house, text, geometry, pot and
presentation data under one aggregate identity. Ordinary profile9 behavior and
snapshot bytes remain the default. Runtime output owns scene/invocation selection;
host art must not infer scene phases from flag combinations. Pot A will be distinct
from resident Interact (browser transport command10 is parent-owned). Box opening
is contact-only. Geometry, contact admission and transition/presentation pacing
must be authenticated compiler inputs, never approximated core floor or timers.

There is one authoritative `StoryFlags` projection. The existing B graph will use
an internal generic storage seam. Its legacy EventFlags/EventSequence aliases stay
unchanged; preserving a borrowed `&EventFlags` getter over sole wide storage is
not possible safely, so the legacy getter will return an owned low-range inspection
projection while `story_flags()` borrows the authority. This is an explicit Rust
getter compatibility change, not a second synchronized flag store.

## Evidence boundary

Foundation tests are synthetic structural tests. They do not establish native
navigation, forced-sequence timing, whole-screen fidelity or browser acceptance.
Those acceptance gates remain with the parent and navigation/compiler owners.
