# Model the plane's own-cell tiles

## Summary

Walking on a world plane (`plane.rs`) ignores own-cell tiles `$88..$8F` (`$80:C440`). The underworld `$03` has none, so nothing breaks yet.

## Dependencies

- [Make the whole underworld playable](underworld-playable.md)

## Requirements

- Model `$80:C440` before a plane with such tiles is played.

## Acceptance Criteria

- A test on a plane cell `$88..$8F` gets the native result.
