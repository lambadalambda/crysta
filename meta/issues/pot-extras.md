# Throw pots while dashing or jumping, and show their fall and fragments

## Summary

The generalized pots (`docs/pots.md`) leave out a dash-throw (released at
once, 6 px a frame) and a jump-throw (the jump's height added), each
measured once natively; the fall a pot shows as an exit drops it
(`$84:C5CB`, stream `$6A`); and the fragments a break leaves (`$84:C7B8`,
frames `$A2:E2BD` onward).

## Dependencies

- [Lift and throw pots as the native game does](throw-pots-freely.md)

## Requirements

- Trace the dash-throw and jump-throw sources and measure them again; draw
  the drop's fall and the break's fragments.

## Acceptance Criteria

- The `throw-right-dashing`, `throw-left-jumping` and `exit-while-carrying`
  fixtures (`local/pots/`) pass frame by frame on both ROMs, and the
  fragments match native frames.

## Progress

- The jump-throw runs (`PotState::throw_in_air`, `world/jump.rs`): the fixture `throw-left-jumping` matches on both ROMs; Ark's `$2A`/`$2B`/`$2C`, stand at J+24, control at J+25.
- The dash-throw runs (`Arc::Dashed`, `Run::Rest`; `a_in_a_carry_dash_is_the_dash_throw`, native `t_held`/`t_free`); the `throw-right-dashing` fixture passes on both ROMs. Its break sound comes a frame early, as the landing's jitter. Not traced: A and B in the dash after it (the model lets A talk or lift, B does nothing), a script's pad lock while its pot flies (the dash runs on), a perpendicular or opposite way or a diagonal in it (ignored).
- Open: the `exit-while-carrying` fixture (the drop's own stream `$2E`, `$84:C6D3`, code only), the fragments.
- The carry dash runs (`room_core::run`'s carry mode, `PotState::step_over_running`; `ark_dashes_with_a_pot_as_natively`, 13 native traces `c_*`). Open: its control word stays the walk's `$20` (natively `$10`); B in it jumps on with the dash (`b_in_a_carry_dash_jumps_on_with_the_dash`; not traced: the ground test from J+24 to J+29, which the model skips, a stream restart after the landing, B again in the landing's frames); on its last frame the model shows the stand pose, natively the walk's.
