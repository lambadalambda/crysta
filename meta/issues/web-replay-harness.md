# Replay European route traces against the deployed web page

## Summary

The European door and post-Box detour routes were replayed in a headless
browser against a locally built page with an uncommitted harness. Commit a
small harness and run both traces against the live GitHub Pages build.

## Dependencies

- [Check the European slice against a native route](european-route.md)

## Requirements

- Load a given page URL in headless Chromium and select the owned ROM through
  the page's file input; the ROM stays local.
- Press Start, drive a controlled clock with one PAL frame per step, and send
  the trace as keydown/keyup events to the page's own handlers.
- Check named checkpoints only through what the page shows: fault, status line
  and canvas. Keep screenshots under `local/`. Play no sound.

## Acceptance Criteria

- Both route traces reach their last checkpoint on
  <https://lambadalambda.github.io/crysta/> with no input mismatch, fault,
  page error or stopped loop, and each checkpoint's canvas matches the Rust
  host's view.

## Notes

- Related: [friend scene](european-friend-scene-freeze.md),
  [post-Box progression](european-post-box-progression.md).
