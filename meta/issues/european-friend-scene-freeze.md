# Reproduce the European friend's door-scene freeze in the player host

## Summary

A player reported that the friends' blue-door scene freezes after they refuse and then accept the request. The report predates the current continuous headless European route test; that test's success does not yet establish that the reported app/browser interaction works.

## Dependencies

- [Accept the European ROM in the app and the web page](european-hosts.md)

## Requirements

- Reproduce or rule out the freeze using the player's actual host/input path, not only direct `World::update` calls.
- Keep the refusal and acceptance branches separate, and preserve the European ROM's own dialogue and choice timing.
- Test the intended lift/throw inputs as exposed by the host; a player reported needing X + Z + direction together.

## Acceptance Criteria

- A headless host-level regression exercises refusal followed by acceptance and releases control without a stuck page, scene, input lock or frozen script.
- The blue door can subsequently be opened by the intended controls and the post-door route remains playable.
- If the report depends on a specific older build, saved state, platform or binding, document the difference and obtain a reproducible case before closing.

## Notes

- The reporter confirmed the **browser** frontend. Its build/version, exact choice presses and any diagnostic output remain unknown.
- The passing continuous European replay takes **direct first-choice acceptance** (`$2E`), not this refusal→retry acceptance (`$2F`). The fresh empty-SRAM native European witness `european_friend_refusal_then_retry_acceptance_releases_ark` refuses (Down, then A) and accepts the retry with real inputs. It reaches `$2F/$3F/$42` at C `(120,448)` around native frame 25026, then acknowledges later pages; `$0B` appears and Ark walks Right to `(184,448)`. A bounded portable European map-C probe seeded only prior `$20/$26/$28` and made the same two choices: despite 15,000 neutral frames, 23 more A presses each separated by 240 frames, and Right44, it remained at `(120,448)` with `$2F/$3F/$42`, **without `$0B`, any page, or directional control**. This is a runtime continuation mismatch, not merely an unsampled keyboard edge, though the seeded portable probe is not yet a checked-in fresh browser route.
- Source lead: native `COP DF` at European `$88:AC7B` installs the player script `$88:AC98`; its later `$88:ACB1..ACC2` instructions grant `$0B` and `$88:ACC7 COP29 $FF50` releases input. The portable `COP DF` currently skips this script operand. Qualify the player-script and child-actor handoffs against native execution before changing lock semantics; the standalone `COP A4` spawn trial did not release the portable scene.
- The other retry result uses a text `$CB` same-bank jump that the decoder currently refuses; it is separate from the reported retry-acceptance branch. Resident `COP A4` is currently length-skipped, although its handler spawns a relative actor on the native `$2F` route.
- Browser input check (headless, owned European ROM): a sampled X hold followed by keyup+keydown between PAL frames was sent as another held X frame with no new edge. The page now passes separately latched keyboard presses to `WebGame`, and repeat keydowns do not latch; a Rust page-turn regression and headless browser frame capture cover this narrow case. This **does not establish** why the reported refusal→retry `$2F` scene remained locked, and the browser route through both choices and the blue door has not yet been verified.
- Input differs by frontend: desktop Space/Enter confirms and X cancels (Z unmapped); browser X confirms and Z cancels. Pots use separate confirm presses to lift and throw; holding X+Z+direction is not required. Host presses are edges, so pressing during typing or holding through the next page does not confirm it.
- Related: [European native route](european-route.md), [scripted movement](scripted-movement-scenes.md).
