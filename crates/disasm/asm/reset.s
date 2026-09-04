; reset.s — initial mixed opaque/annotated ROM reconstruction.
;
; Opaque bytes remain local in rom-clean.bin. The first reset instructions are
; assembled explicitly to prove that known ranges can replace bounded .incbin
; slices without changing placement.

.setcpu "65816"
.import __ROM_SIZE__
.segment "ROM"

.incbin "rom-clean.bin", $000000, $008000

; Hardware reset enters through the $00:8000 mirror in emulation mode:
; E=1, M=1, X=1, PBR=$00, DBR=$00, and direct page=$0000.
.a8
.i8
NativeResetEntry = $808017

Reset:
.assert Reset = $C08000, lderror, "reset vector target moved"
    sei
    clc
    xce
    jml NativeResetEntry
ResetPrefixEnd:
.assert ResetPrefixEnd - Reset = 7, error, "reset prefix size changed"

.incbin "rom-clean.bin", $008007, $3F7FF9

.assert __ROM_SIZE__ = $400000, lderror, "reconstructed ROM size changed"
