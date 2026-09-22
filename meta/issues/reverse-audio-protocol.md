# Reverse the CPU-to-SPC audio protocol

## Summary

Document audio bootstrap, command transport, sequence/sample inputs, and timing before selecting the permanent compatibility backend.

## Dependencies

- [Classify ROM code, data, and indirect dispatch](classify-rom-code-data.md)
- [Export and compare reference frame state](reference-state-export.md)

## Requirements

- Locate SPC700 program upload and initialization from the main CPU.
- Identify CPU/APU port commands, acknowledgements, queues, and timing assumptions.
- Map locally extracted APU RAM, DSP state, sequences, and samples to source ROM regions.
- Define audio-command traces suitable for deterministic comparison without committing captured copyrighted data.
- Document which behavior belongs in the portable core, audio compatibility layer, and platform frontend.

## Acceptance Criteria

- Boot and at least one music and sound-effect transition have annotated command traces.
- A local tool can identify or reconstruct the initial APU image from a verified ROM.
- Command and timing semantics needed by the opening slice are documented.
- The later SPC backend issue has bounded, testable integration inputs.

## Notes

- Milestone: [M3 — Content and script pipeline](../milestones.md#m3-content-and-script-pipeline)
- This issue covers reverse engineering; backend library selection and playback integration remain separate.
- [Bounded native Crysta work](native-crysta-music.md) now documents source-only
  driver/selection3/sample extraction and physical IPL/resident uploads, including
  the receiver's ACK-before-read settling requirement. See the
  [source recipe](../../tools/native-music-qualification/README.md). This is not
  a complete command trace, sound-effect transition or whole-protocol qualification.
