# Qualify the first exterior landing profile

## Summary

Source-project the first exterior map reached after the room B progression conversation: background resources, camera, collision profile and a bounded landing/walking area. Inventory but do not implement every outdoor actor or later exit.

## Dependencies

- [Complete the fresh house scene with all residents](complete-house-scene-setup.md)

## Requirements

- Bounded subissue of [talk and leave the house](talk-and-leave-house.md).
- ROM/source-driven compilation, no capture-seeded production data or original CPU in simulation.
- Independent correctness/architecture review before substantial commits.

## Acceptance Criteria

- Source/native evidence qualifies the destination loading recipe, collision/camera and landing area; extracted resources remain local. Gated or unsupported map behavior rejects explicitly.

## Completion

- MapA's distinct64x80 source sheet and1024x1024 camera bounds are qualified. First logical background independently resolves hardwareBG2/mode09, preserving opaque-highBG2/OBJ2/lowBG2 ordering.
- Three fresh settled checkpoints match672 full BG2 tile words and43,008 base index/priority pixels each. The42-cell sample halo [29,47,36,53] exactly matches native collision words; no new material is needed for Down/Right landing movement. Wider terrain/actors are not admitted.
- Parent separately ran the source/native driver at ignored `local/house-exterior-qualification/run-BFoR7h` against fresh conversation replay-OYXAH7. Assets tests, synthetic checker mutations, normal/optimized checks and independent reviews passed. See `docs/house-exterior.md` for camera, halo and omitted secondary effects.
- Core sample-bound enforcement, host bitmap selection and actual browser departure remain umbrella integration gates; source decoding alone does not admit all of Crysta.
