# Finish the underworld

## Summary

`$12A` (Mu), `$12B` (Polynesia), the elder setting `$74`, the Hole `$127` with its Yes/No choice, and the change to Chapter 2.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Model the last maps and the chapter change.

## Acceptance Criteria

- The chapter ends as natively.

## Progress

- The continents' doors (Mu tested; Polynesia after its enemies): the reload, the text, the flags and the way back; the parchment's picture is not drawn.
- The Hole: the conditional exit with `$74`, Elle's farewell (`$247`), the elder's choice, the rim's lips and a fixed jump into the exit.
- The chapter's end: a dark stand-in for the vortex, the title text (Japanese; the European one is refused by the page geometry and left out), flag `$06F`, and `World::chapter_over`.
