# Warping the reference emulator to a map

To trace a map the journey states do not reach, load a state on a map with
ordinary control and write the transfer request into WRAM, as `COP 14` does
(`docs/new-game-bootstrap.md`): `$047C` the map, `$0484` the mode, `$0490`
the selector, `$0492`/`$0494` the raw position (the loader adds (8,16)).
The next frames load the map as a door would.

The oracle has no WRAM write. A scratch probe finds the 128 KiB WRAM image
(`Session::wram_image`) once in `oracle::save_state`'s bytes, patches the
words there and loads the state back with `oracle::load_state`.

Used for the lip drop on `$10F` (2026-10-08): JP
`local/pandora-tower-discovery/departure/journey/tower-align-east-rest.state`
(map `$03`), `$047C = $10F`, mode 0, selector 0, raw (128,720): Ark stands
at (136,736) from frame 60477. Read Ark through `$0DEA` (+`$00`/`+$02`
place, `$7F:1008,X` list, `$7F:100A,X` composition), `$097C`, `$0956`,
and the port writes (`Session::take_apu_port_writes`).
