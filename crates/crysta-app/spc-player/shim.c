/* Project-authored MIT shim; see README.md for upstream provenance/patches. */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "apu.h"

/* One checked allocation avoids upstream's unchecked malloc constructors.
 * Keep the small pointer wiring in sync with apu_init/spc_init/dsp_init. */
typedef struct {
  Apu apu;
  Spc spc;
  Dsp dsp;
  uint16_t read_offset;
} Player;

void* spc_player_new(void) {
  Player* p = calloc(1, sizeof(Player));
  if (!p) return NULL;
  p->apu.snes = NULL; /* Never call the frame-oriented dsp_getSamples. */
  p->apu.spc = &p->spc;
  p->apu.dsp = &p->dsp;
  p->spc.mem = &p->apu;
  p->spc.read = apu_spcRead;
  p->spc.write = apu_spcWrite;
  p->spc.idle = apu_spcIdle;
  p->dsp.apu = &p->apu;
  apu_reset(&p->apu);
  return p;
}

void spc_player_free(void* player) {
  /* Embedded components: do not call upstream's individual frees. */
  free(player);
}

void spc_player_write_port(void* player, size_t index, uint8_t value) {
  Player* p = player;
  p->apu.inPorts[index] = value;
}

uint8_t spc_player_read_port(const void* player, size_t index) {
  const Player* p = player;
  return p->apu.outPorts[index];
}

void spc_player_read_ram(const void* player, size_t start, uint8_t* out, size_t len) {
  const Player* p = player;
  if (len) memcpy(out, p->apu.ram + start, len);
}

/* The pinned SPC executes one finite opcode (or one idle cycle when stopped).
 * Audit bound: <= 32 cycles, hence <= one DSP sample. Check progress even if a
 * later vendor update changes that contract. Unsigned subtraction handles the
 * APU's uint32_t cycle counter wrap. */
static uint32_t step(Player* p) {
  uint32_t before = p->apu.cycles;
  spc_runOpcode(&p->spc);
  uint32_t elapsed = p->apu.cycles - before;
  return elapsed > 0 && elapsed <= 32 ? elapsed : 0;
}

int32_t spc_player_run_cycles(void* player, uint32_t cycles) {
  Player* p = player;
  uint32_t elapsed = 0;
  p->read_offset = p->dsp.sampleOffset;
  while (elapsed < cycles) {
    uint32_t advanced = step(p);
    if (!advanced) return -1;
    elapsed += advanced;
    /* Drain after EACH opcode, not after a bulk run that can wrap the ring. */
    p->read_offset = p->dsp.sampleOffset;
  }
  return (int32_t)elapsed;
}

int32_t spc_player_render(void* player, int16_t* out, size_t frames) {
  Player* p = player;
  /* At least one cycle/opcode, one DSP sample/32 cycles, plus start phase.
   * The public Rust boundary bounds frames, so this cannot overflow. */
  size_t fuel = frames * 32 + 32;
  for (size_t frame = 0; frame < frames; frame++) {
    while (p->read_offset == p->dsp.sampleOffset) {
      if (!fuel || !step(p)) return -1;
      fuel--;
    }
    /* Both offsets wrap at 16 bits. Drain before executing another opcode,
     * preserving unread output across calls, including empty renders. */
    size_t offset = (p->read_offset & 0x3ff) * 2;
    out[frame * 2] = p->dsp.sampleBuffer[offset];
    out[frame * 2 + 1] = p->dsp.sampleBuffer[offset + 1];
    p->read_offset++;
  }
  return 0;
}
