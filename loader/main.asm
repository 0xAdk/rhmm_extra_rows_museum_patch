.arm.little

.open "input/code.bin", "output/code.bin", 0x100000

@patch_loader_injection_loc equ 0x100010
.org @patch_loader_injection_loc
	b load_patch_detour

@free_space_start equ 0x399C00
@free_space_end   equ 0x39A000

.org @free_space_start
.area @free_space_end - .

.func load_patch_detour
	push {r0-r12}
	bl load_patch
	pop {r0-r12}

	blx 0x10097C
	b @patch_loader_injection_loc + 4
.endfunc

injection_filepath:     .asciiz "/luma/titles/000400000018A400/injection.bin"
injection_filepath_size equ . - injection_filepath
.align

.func load_patch
	push {lr}

	ldr r0, =injection_filepath
	ldr r1, =injection_filepath_size
	bl load_file_rwx

	// Keep where the rust injection code is allocated in memory in the r11
	// register. So I can figure out where the rust code crashed. Not the best
	// solution but currently the rust code doesn't touch r11, so it works out
	// (for now)
	mov r11, r0
	blx r0

	pop {pc}
.endfunc

@FS_SERVICE_INIT     equ 0x28B3BC
@FS_SERVICE_HANDLE   equ 0x54DD18
@OPEN_FILE_DIRECTLY  equ 0x279E60
@GET_FILE_SIZE       equ 0x2BC628
@MALLOC              equ 0x28C108
@READ_FILE           equ 0x2BC544
@CLOSE_FILE          equ 0x2BC59C
.func load_file_rwx
	push {r1-r7, lr}
	sub sp, 0x24

	mov r6, r0 // file path
	mov r7, r1 // file path size

	// sets @FS_SERVICE_HANDLE
	bl @FS_SERVICE_INIT

	// open file
	ldr r0, =@FS_SERVICE_HANDLE
	add r1, sp, 0x20   // pointer to output file handle
	mov r2, 0          // transaction         = 0
	mov r3, 9          // archive id          = SDMC
	mov r4, 1
	str r4, [sp, 0x00] // archive path type    = EMPTY
	str r2, [sp, 0x04] // archive data pointer = NULL
	str r2, [sp, 0x08] // archive path size    = 0
	mov r5, 3
	str r5, [sp, 0x0C] // filepath type        = ASCII
	str r6, [sp, 0x10] // file data pointer
	str r7, [sp, 0x14] // filepath size
	str r4, [sp, 0x18] // file open flags      = READ
	str r2, [sp, 0x1C] // attributes           = 0
	bl @OPEN_FILE_DIRECTLY

	// get filesize
	add r0, sp, 0x20 // r0 = pointer to file handle
	add r1, sp, 0x10 // r1 = pointer to file size
	bl @GET_FILE_SIZE

	ldr r0, [sp, 0x10]
	bl @MALLOC            // allocate space for file copy
	bl mark_memory_as_rwx // mark allocated memory as RWX
	mov r7, r0 // keep the adress of the allocated memory to be returned

	str r0, [sp]       // output buffer
	add r0, sp, 0x20   // pointer to file handle
	add r1, sp, 8      // pointer to bytes read output
	mov r2, 0          // file offset (lower word)
	mov r3, 0          // file offset (higher word)
	ldr r4, [sp, 0x10]
	str r4, [sp, 0x04] // buffer size
	bl @READ_FILE

	add r0, sp, 0x20
	bl @CLOSE_FILE

	add sp, 0x24
	mov r0, r7 // return the address to the start of the memory that got allocated
	pop {r1-r7, pc}
.endfunc

@CURRENT_PROCESS_PSEUDO_HANDLE equ 0xFFFF8001
.func mark_memory_as_rwx
	push {r0-r6, lr}

	// 0x2: QueryMemory(address[r2]) -> (base_process_addr[r1], size[r2])
	mov r2, r0
	swi 0x2
	push {r1, r2} // {mem.base_address, mem.size}

	// 0x35: GetProcessId(handle[r1]) -> process_id[r1]
	// 0x33: OpenProcess(process_id[r1]) -> handle[r1]
	ldr r1, =@CURRENT_PROCESS_PSEUDO_HANDLE
	swi 0x35
	swi 0x33
	mov r6, r1

	// ControlProcessMemory(
	//     handle[r0],
	//     addr0[r1],
	//     addr1[r2],
	//     size[r3],
	//     type[r4],
	//     perm[r5]
	// )

	mov r0, r6    // process handle
	mov r2, 0     // addr2 = NULL
	pop {r1, r3}  // {mem.base_address, mem.size}
	ldr r4, =6    // type = MEMOP_PROT
	ldr r5, =7    // perm = MEMPERM_RWX
	swi 0x70

	// CloseHandle(handle[r0])
	mov r0, r6    // process handle
	swi 0x23

	pop {r0-r6, pc}
.endfunc

.pool

.endarea
.close
