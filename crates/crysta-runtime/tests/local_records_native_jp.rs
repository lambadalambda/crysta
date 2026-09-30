//! The native game loads the slot the desk's Records screen wrote; one
//! ares session a process, so one ROM a test binary.
#[path = "support/desk.rs"]
mod desk;

#[test]
fn the_native_game_loads_the_desk_save() {
    let name = "Tenchi Souzou (Japan).sfc";
    let Some(rom) = desk::load(name) else {
        return;
    };
    let mut world = desk::save_at_desk(rom.image());
    desk::assert_desk_slot(&world, name);
    desk::assert_sram_written_once(&mut world);
    let session = desk::native_load(&rom, world.sram().bytes());
    desk::assert_loaded_at_desk(&session, name);
}
