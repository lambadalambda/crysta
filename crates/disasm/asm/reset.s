; reset.s — Initial fully-opaque ROM reconstruction.
;
; This file includes the entire verified ROM via .incbin, providing a
; byte-for-byte baseline. As ranges are disassembled, .incbin segments
; shrink and real 65816 assembly takes their place.

.segment "ROM"

; Path is injected by the build script via --define or by placing the
; clean ROM in OUT_DIR. For now, the build script writes a clean ROM
; to OUT_DIR/rom-clean.bin and we include it with a relative path.
; ca65's -o flag puts the .o in OUT_DIR, so the include path is
; relative to there.

.incbin "rom-clean.bin"
