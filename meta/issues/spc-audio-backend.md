# Integrate a compatible SPC audio backend

## Summary

Play original music and sound effects with correct command and timing behavior while isolating audio from simulation.

## Dependencies

- [Select the project license and contribution policy](select-project-license.md)
- [Reverse the CPU-to-SPC audio protocol](reverse-audio-protocol.md)
- [Build the reproducible local asset pack](local-asset-pack.md)

## Requirements

- Evaluate licensed SPC700/S-DSP implementations.
- Extract or initialize required audio program and sequence data locally.
- Map portable audio commands to the compatibility backend.
- Handle browser audio-resume restrictions in the frontend layer.

## Acceptance Criteria

- Opening and first-tower music and effects play without simulation timing changes.
- Audio output is stable under replay and pause/resume.
- Third-party licensing and attribution are documented.

## Notes

- Milestone: [M5 — Classic presentation and Chapter 1](../milestones.md#m5-classic-presentation-and-chapter-1)
- A high-level music reimplementation is optional future work, not required for classic fidelity.
- [Native Crysta music](native-crysta-music.md) adds a bounded MIT LakeSnes
  audio-only backend and rodio device integration, with source-derived selection3.
  Software/PCM tests pass; the user confirmed audible desktop playback.
  Map/event track changes, effects, browser audio and classic timing remain open.
