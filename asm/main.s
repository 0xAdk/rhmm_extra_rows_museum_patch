.arm.little
.open "input/code.bin", "output/code.bin", 0x100000

loop_museum equ 0x399C00

; right before MuseumScene::get_next_row returns -1 for "row not found"
.org 0x002424a8
	bl loop_museum

.org loop_museum
	cmp r9, 0x0  ; 0 = SearchDirection::UP
	             ; 1 = SearchDirection::DOWN

	moveq r0, 0  ; first row
	movne r0, 28 ; last row

	bx lr
