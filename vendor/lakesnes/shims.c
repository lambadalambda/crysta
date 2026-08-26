/* LakeSnes helper shims: expose struct members the Rust FFI wants to read.
 * The core struct is defined in snes.h; these are thin accessors. */
#include <stdint.h>
#include <stdbool.h>
#include "snes.h"

const uint8_t* snes_ram(const Snes* snes) { return snes->ram; }
uint32_t snes_frames(const Snes* snes) { return snes->frames; }
uint64_t snes_cycles(const Snes* snes) { return snes->cycles; }

/* VRAM/CGRAM/OAM access for the headless oracle. */
const uint16_t* snes_vram(const Snes* snes) { return snes->ppu->vram; }
const uint16_t* snes_cgram(const Snes* snes) { return snes->ppu->cgram; }

/* CPU register access for the headless oracle. */
uint16_t snes_cpu_pc(const Snes* snes) { return snes->cpu->pc; }
uint8_t snes_cpu_bank(const Snes* snes) { return snes->cpu->k; }
