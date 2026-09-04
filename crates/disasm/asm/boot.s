; boot.s — reset, native interrupt entry, frame gate, and main dispatch.
;
; The file owns the complete ascending 4 MiB image. Bounded opaque slices keep
; unknown regions local while named islands assemble at exact canonical HiROM
; addresses. Runtime long addresses use the $80-$BF mirrors encoded by the ROM.

.setcpu "65816"
.import __ROM_LOAD__, __ROM_SIZE__
.segment "ROM"

ROM_BASE = $C00000
ROM_SIZE = $400000

NmiHandlerRuntime = $85F98F
IrqHandlerRuntime = $85FB00
BrkHandlerRuntime = $85FB01
CopHandlerRuntime = $808378
NativeResetRuntime = $808017
WaitForFrameAndPollInput = $868000
StateHandlerPointer = $049E

RomBegin:
.assert RomBegin = ROM_BASE, lderror, "ROM start moved"
.incbin "rom-clean.bin", $000000, $008000

; Hardware reset enters through the $00:8000 mirror in emulation mode:
; E=1, M=1, X=1, PBR=$00, DBR=$00, direct page=$0000, IRQ masked.
.a8
.i8
Reset:
.assert Reset = $C08000, lderror, "reset vector target moved"
    sei
    clc
    xce                         ; E=0, C=1; M/X remain 8-bit
    jml NativeResetRuntime

; Native vector trampolines execute in bank $00 and establish handler PBRs.
NmiTrampoline:
    jml NmiHandlerRuntime
IrqTrampoline:
    jml IrqHandlerRuntime
CopTrampoline:
    jml CopHandlerRuntime
BrkTrampoline:
    jml BrkHandlerRuntime

; Entry: E=0, M=1, X=1, PBR=$80, DBR=$00, D=$0000.
NativeReset:
.assert NativeReset = $C08017, lderror, "native reset entry moved"
    cld
    rep #$30
.a16
.i16
    lda #$0000
    tcd                          ; direct page = $0000
    lda #$01FF
    tcs                          ; native stack = $01FF
    sep #$20
.a8
    lda #$81
    pha
    plb                          ; DBR=$81 (low WRAM mirror)

    jsl $86B9CB                 ; initialize hardware from table at $86:B9E8
    jsl $86B8D6                 ; clear WRAM and initialize low-WRAM fields
    jsl $86AA9C                 ; upload the initial SPC driver/data
    lda #$01
    sta $047C
    stz $0484
    inc $0488
    jsl $8D86F8                 ; initialize the opening program state

; One iteration is gated by one completed NMI period, then runs common updates
; before dispatching through the mutable 16-bit state-handler pointer at $049E.
MainLoop:
.assert MainLoop = $C08043, lderror, "main loop moved"
    jsl WaitForFrameAndPollInput
    jsr $820C
    jsl $8D9328
    jsl $80E8AF
    jsl $8D86F8
    jsl $8D8797
TopLevelStateDispatch:
.assert TopLevelStateDispatch = $C0805A, lderror, "state dispatch moved"
    jmp (StateHandlerPointer)
BootMainEnd:
.assert BootMainEnd - Reset = $005D, error, "boot/main island size changed"

.incbin "rom-clean.bin", $00805D, $008378 - $00805D

; Native COP is a software-service dispatcher. The signature byte immediately
; before the stacked return address indexes a 16-bit handler table at $83B2.
; Service handlers return through one of the helpers below after advancing the
; stacked return address past their inline operands. ABI: E=0, X=0 (16-bit),
; and D=$0000 are required; M and DBR are inherited, then M is forced to 0.
; Dispatch clobbers A/X/Y and direct-page scratch $36/$38 before transferring
; control to the selected service.
.a16
.i16
NativeCopHandler:
.assert NativeCopHandler = $C08378, lderror, "COP handler moved"
    rep #$20
    txy
    lda $04,s
    sta $38
    lda $02,s
    dec a
    sta $36
    lda [$36]
    inc $36
    and #$00FF
    asl a
    tax
    jmp ($83B2,x)
CopReturnAfterTwoBytes:
.assert CopReturnAfterTwoBytes = $C08390, lderror, "COP 2-byte return moved"
    lda $36
    inc a
    inc a
    sta $02,s
    rti
CopReturnAfterFourBytes:
.assert CopReturnAfterFourBytes = $C08397, lderror, "COP 4-byte return moved"
    lda $36
    clc
    adc #$0004
    sta $02,s
    rti
CopReturnAfterFiveBytes:
.assert CopReturnAfterFiveBytes = $C083A0, lderror, "COP 5-byte return moved"
    lda $36
    clc
    adc #$0005
    sta $02,s
    rti
CopReturnAfterEightBytes:
.assert CopReturnAfterEightBytes = $C083A9, lderror, "COP 8-byte return moved"
    lda $36
    clc
    adc #$0008
    sta $02,s
    rti
CopDispatchTable:
.assert CopDispatchTable = $C083B2, lderror, "COP dispatch table moved"

.incbin "rom-clean.bin", $0083B2, $00FFE4 - $0083B2

; Native-mode vectors. ABORT and the reserved slot are zero and unsupported.
NativeVectors:
.assert NativeVectors = $C0FFE4, lderror, "native vectors moved"
    .word .loword(CopTrampoline)
    .word .loword(BrkTrampoline)
    .word $0000
    .word .loword(NmiTrampoline)
    .word $0000
    .word .loword(IrqTrampoline)
NativeVectorsEnd:
.assert NativeVectorsEnd - NativeVectors = $000C, error, "native vector table size changed"

; These non-native slots are retained exactly. Only emulation RESET is used;
; reset immediately switches to native mode before interrupts are enabled.
    .word $FF7E, $FEFE
EmulationVectors:
.assert EmulationVectors = $C0FFF4, lderror, "emulation vectors moved"
    .word $BFF7                ; COP: unqualified/unused
    .word $AFFF                ; reserved
    .word $F9FB                ; ABORT: unqualified/unused
    .word $FFFD                ; NMI: unqualified/unused
    .word .loword(Reset)
    .word $F5FF                ; IRQ/BRK: unqualified/unused
RomBankC0End:
.assert RomBankC0End = $C10000, lderror, "bank C0 size changed"

.incbin "rom-clean.bin", $010000, $05F98F - $010000

; Native NMI entry: M/X are inherited until REP establishes 16-bit widths.
; The handler preserves P, DBR, A, X, Y, and D; uses DBR=$81 and D=$0000;
; disables HDMA; runs the queued transfer setup; and starts a 512-byte DMA from
; $7F:0600 to CGRAM before performing the remaining per-vblank updates.
NativeNmiHandler:
.assert NativeNmiHandler = $C5F98F, lderror, "NMI handler moved"
    php
    phb
    rep #$30
.a16
.i16
    pha
    phx
    phy
    phd
    cld
    lda #$0000
    tcd
    sep #$20
.a8
    lda #$81
    pha
    plb
    stz $420C                   ; disable HDMA during NMI transfers
    jsl $86A505
    stz $2121                   ; CGRAM destination index 0
    stz $4300                   ; DMA mode 0, CPU -> PPU
    lda #$22
    sta $4301                   ; destination $2122 (CGRAM data)
    ldx #$0600
    stx $4302
    lda #$7F
    sta $4304                   ; source $7F:0600
    ldx #$0200
    stx $4305                   ; 512 bytes
    lda #$01
    sta $420B                   ; start DMA channel 0
NmiKnownPrefixEnd:
.assert NmiKnownPrefixEnd = $C5F9CA, lderror, "NMI prefix size changed"

.incbin "rom-clean.bin", $05F9CA, $05FAB9 - $05F9CA

; NMI tail restores HDMA, waits for HBlank to end, captures both controller
; ports, optionally updates the APU port, advances two frame counters, restores
; the complete entry context, and returns from interrupt.
.a8
.i16
NmiTail:
.assert NmiTail = $C5FAB9, lderror, "NMI tail moved"
    jsr $FB08
    lda $86
    sta $420C
NmiWaitForHblankEnd:
    lda $4212
    ror a
    bcs NmiWaitForHblankEnd
    rep #$20
.a16
    stz $84
    lda $4218
    sta $0456                   ; controller port 1
    lda $421A
    sta $0458                   ; controller port 2
    lda $04B8
    beq @apuUpdate
    bmi @finish
    jsl $80818F
    bra @finish
@apuUpdate:
    lda $42
    lsr a
    lda #$0000
    bcs @writeApuPort
    lda $04B6
    stz $04B6
@writeApuPort:
    sta $2142
@finish:
    inc $42
    inc $44
    pld
    ply
    plx
    pla
    plb
    plp                          ; restores inherited M/X
NativeNmiReturn:
.assert NativeNmiReturn = $C5FAFF, lderror, "NMI return moved"
    rti

; Native IRQ is intentionally ignored. Native BRK emits the emulator/debugger
; marker store used by the original program and then resumes the interrupted
; context. Both inherit M/X, DBR, and direct page.
NativeIrqHandler:
.assert NativeIrqHandler = $C5FB00, lderror, "IRQ handler moved"
    rti
NativeBrkHandler:
.assert NativeBrkHandler = $C5FB01, lderror, "BRK handler moved"
    nop
    nop
    sta f:$FF8000
    rti
InterruptIslandEnd:
.assert InterruptIslandEnd = $C5FB08, lderror, "interrupt island size changed"

.incbin "rom-clean.bin", $05FB08, $068000 - $05FB08

; Called once at the start of every main-loop iteration. The first read clears
; a stale NMI latch; the loop then waits until bit 7 of RDNMI ($4210) reports a
; new NMI period. The opaque remainder normalizes controller input and returns.
; Entry from MainLoop: E=0, M=1, X=0, PBR=$86, DBR=$81, D=$0000.
.a8
.i16
WaitForFrameAndPollInputBody:
.assert WaitForFrameAndPollInputBody = $C68000, lderror, "frame gate moved"
    php
    sep #$20
.a8
    pha
    phy
    lda f:$004210
WaitForNmiLatch:
    lda f:$004210
    bpl WaitForNmiLatch
    lda f:$004210
    rep #$20
.a16
FrameGateKnownPrefixEnd:
.assert FrameGateKnownPrefixEnd = $C68015, lderror, "frame gate prefix size changed"

.incbin "rom-clean.bin", $068015, ROM_SIZE - $068015
RomEnd:
.assert RomEnd - RomBegin = ROM_SIZE, error, "ROM stream size changed"
.assert __ROM_LOAD__ = ROM_BASE, lderror, "ROM link address changed"
.assert __ROM_SIZE__ = ROM_SIZE, lderror, "reconstructed ROM size changed"
