#![no_std]
#![no_main]
#![feature(stmt_expr_attributes, const_trait_impl)]

use core::{arch::asm, ptr::addr_of};

#[export_name = "_start"]
#[link_section = ".text._start"]
fn setup_hooks() {
	unsafe {
		run_with_text_rw(move || {
			#[rustfmt::skip]
			let museum_row_pointer_addresses = [
				0x2421E8, 0x2421E8, 0x24C480, 0x24D408,           // gRowInfo
				0x1979E0, 0x1983FC, 0x242350, 0x2424BC, 0x24E010, // gRowInfo1
				0x2423D4, 0x2619C0,                               // gRowInfo2
				0x224FE8, 0x225008, 0x225024, 0x225044, 0x225068, // gRowInfo3
			];

			// each address is a pointer to one of the 4 musume row info arrays
			// replace each one with a pointer to MY_MUSEUM_ROWS
			for address in museum_row_pointer_addresses {
				*(address as *mut u32) = addr_of!(MY_MUSEUM_ROWS) as _;
			}

			// see section F5.1.35 in the arm A-profile reference manual
			const fn make_cmp_immediate_instruction(register: u32, value: u32) -> u32 {
				// register is 4 bits, value is 12 bits
				assert!(register < 2_u32.pow(4));
				assert!(value < 2_u32.pow(12));

				// 1110 == no condition
				let cond = 0b1110 << 28;
				let cmp_imm_base = 0b0000_00110_10_1 << 20;
				let register = register << 16;

				cond | cmp_imm_base | register | value
			}

			let compare_r1_instruction: u32 = // cmp r1, MUSEUM_ROW_COUNT
				make_cmp_immediate_instruction(1, MUSEUM_ROW_COUNT as u32);

			let compare_r8_instruction: u32 = // cmp r8, MUSEUM_ROW_COUNT
				make_cmp_immediate_instruction(8, MUSEUM_ROW_COUNT as u32);

			// replace conditions in loops that loop over the museum rows
			// with the new MUSEUM_ROW_COUNT

			// find_column_with_game
			*(0x2423C4 as *mut u32) = compare_r1_instruction;

			// get_next_row
			*(0x2423DC as *mut u32) = compare_r1_instruction;
			*(0x242400 as *mut u32) = compare_r8_instruction;
			*(0x2424A0 as *mut u32) = compare_r8_instruction;

			// find_row_with_game
			*(0x2619B0 as *mut u32) = compare_r1_instruction;

			// redirect museum row color array to our one
			let museum_row_color_pointer_addresses =
				[0x17D8B4, 0x17E318, 0x17E4C4, 0x241FC4, 0x38E6D8];

			for address in museum_row_color_pointer_addresses {
				*(address as *mut u32) = addr_of!(MY_MUSEUM_ROW_COLORS) as _;
			}

			Ok(())
		})
		.unwrap();
	}
}

unsafe fn run_with_text_rw(f: impl Fn() -> Result<(), SvcResult>) -> Result<(), SvcResult> {
	let text_start = 0x100000 as *const ();
	let text_size = 0x29A000;

	let process_handle_wrapper = open_current_process_handle()?;

	process_memory_set_permissions(
		process_handle_wrapper.handle,
		text_start,
		text_size,
		MemoryPermission::RWX,
	)?;

	f()?;

	process_memory_set_permissions(
		process_handle_wrapper.handle,
		text_start,
		text_size,
		MemoryPermission::RX,
	)?;

	Ok(())
}

type SvcResult = u32;

#[allow(dead_code)]
struct MemoryInfo {
	base_addr: *const (),
	size: usize,
	perm: MemoryPermission,
}

type Handle = u32;
struct HandleWrapper {
	handle: Handle,
}

impl Drop for HandleWrapper {
	fn drop(&mut self) {
		let _ = close_handle(self.handle);
	}
}

#[repr(u32)]
#[allow(dead_code, clippy::upper_case_acronyms)]
enum MemoryPermission {
	None = 0,
	R = 1,
	W = 2,
	RW = 3,
	X = 4,
	RX = 5,
	WX = 6,
	RWX = 7,
	DontCare = 0x10000000,
}

fn process_memory_set_permissions(
	process_handle: Handle,
	addr: *const (),
	size: usize,
	perm: MemoryPermission,
) -> Result<(), SvcResult> {
	let mut result: SvcResult;
	unsafe {
		asm!(
			"
			swi 0x70
			",
			in("r0") process_handle,
			in("r1") addr,
			in("r2") core::ptr::null::<()>(),
			in("r3") size,
			in("r4") MemoryOperation::Protect as u32,
			in("r5") perm as u32,

			lateout("r0") result,

			// clobber
			lateout("r1") _,
			lateout("r2") _,
			lateout("r3") _,
			lateout("r12") _,
		)
	}

	if result == 0 {
		Ok(())
	} else {
		Err(result)
	}
}

#[allow(dead_code)]
enum MemoryOperation {
	Free = 1,
	Reserve = 2,
	Commit = 3,
	Map = 4,
	Unmap = 5,
	Protect = 6,
	RegionApp = 0x100,
	RegionSystem = 0x200,
	RegionBase = 0x300,
	Linear = 0x10000,
}

const CURRENT_PROCESS_PSEUDO_HANDLE: Handle = 0xFFFF8001;
fn open_current_process_handle() -> Result<HandleWrapper, SvcResult> {
	let mut result: SvcResult;
	let mut process_handle: Handle;

	unsafe {
		asm!(
			r#"
			swi 0x35 // get_process_id(handle[r1]) -> process_id[r1]
			swi 0x33 // open_process(process_id[r1]) -> handle[r1]
			"#,
			out("r0") result,
			inlateout("r1") CURRENT_PROCESS_PSEUDO_HANDLE => process_handle,

			// clobber
			lateout("r2") _,
			lateout("r3") _,
			lateout("r12") _,
		)
	}

	if result == 0 {
		Ok(HandleWrapper {
			handle: process_handle,
		})
	} else {
		Err(result)
	}
}

fn close_handle(handle: Handle) -> Result<(), SvcResult> {
	let mut result: SvcResult;

	unsafe {
		asm!(
			"swi 0x23",
			inlateout("r0") handle => result,

			// clobber
			lateout("r1") _,
			lateout("r2") _,
			lateout("r3") _,
			lateout("r12") _,
		)
	}

	if result == 0 {
		Ok(())
	} else {
		Err(result)
	}
}

#[repr(C)]
struct MuseumRow {
	column_count: usize,
	game_indices: [u16; 5],
	pad: [u8; 2],
	title_id: u32,
	high_index: u32,
	low_index: u32,
}

impl MuseumRow {
	const fn new(game_indices: [u16; 5], title_id: u32, high_index: u32, low_index: u32) -> Self {
		let column_count = match () {
			_ if game_indices[1] == 0x101 => 1,
			_ if game_indices[3] == 0x101 => 3,
			_ if game_indices[4] == 0x101 => 4,
			_ => 5,
		};

		Self {
			column_count,
			game_indices,
			pad: [0, 0],
			title_id,
			high_index,
			low_index,
		}
	}
}

const MUSEUM_ROW_COUNT: usize = 32;

#[rustfmt::skip]
static MY_MUSEUM_ROWS: [MuseumRow; MUSEUM_ROW_COUNT] = [
	/* E2 */ MuseumRow::new([0x049, 0x04a, 0x02c, 0x101, 0x101], 0x50201b, 0, 0),
	/* E1 */ MuseumRow::new([0x069, 0x068, 0x06b, 0x06a, 0x101], 0x50201b, 0, 0),
	/* E0 */ MuseumRow::new([0x023, 0x05b, 0x006, 0x00e, 0x05d], 0x50201b, 0, 0),

	/* 0  */ MuseumRow::new([0x059, 0x005, 0x007, 0x00d, 0x101], 0x50215a, 0, 0),
	/* 1  */ MuseumRow::new([0x002, 0x003, 0x00a, 0x00b, 0x101], 0x50217e, 0, 1),
	/* 2  */ MuseumRow::new([0x069, 0x101, 0x101, 0x101, 0x101], 0x502141, 0, 2),
	/* 3  */ MuseumRow::new([0x000, 0x006, 0x008, 0x00c, 0x101], 0x5021a2, 0, 3),
	/* 4  */ MuseumRow::new([0x017, 0x01d, 0x02d, 0x044, 0x101], 0x5021bb, 0, 4),
	/* 5  */ MuseumRow::new([0x068, 0x101, 0x101, 0x101, 0x101], 0x502165, 0, 5),
	/* 6  */ MuseumRow::new([0x001, 0x004, 0x009, 0x00e, 0x101], 0x5021c6, 0, 6),
	/* 7  */ MuseumRow::new([0x019, 0x01f, 0x02e, 0x045, 0x101], 0x5021d1, 0, 7),
	/* 8  */ MuseumRow::new([0x06b, 0x101, 0x101, 0x101, 0x101], 0x502189, 0, 8),
	/* 9  */ MuseumRow::new([0x00f, 0x025, 0x034, 0x04a, 0x05e], 0x502033, 0, 8),
	/* 10 */ MuseumRow::new([0x05a, 0x027, 0x02c, 0x049, 0x060], 0x501ff3, 0, 0),
	/* 11 */ MuseumRow::new([0x012, 0x022, 0x038, 0x046, 0x061], 0x501ffb, 0, 1),
	/* 12 */ MuseumRow::new([0x010, 0x028, 0x032, 0x047, 0x062], 0x502003, 0, 2),
	/* 13 */ MuseumRow::new([0x018, 0x024, 0x037, 0x043, 0x063], 0x50200b, 0, 3),
	/* 14 */ MuseumRow::new([0x011, 0x026, 0x035, 0x04d, 0x064], 0x502013, 0, 4),
	/* 15 */ MuseumRow::new([0x01b, 0x023, 0x036, 0x04b, 0x065], 0x50201b, 0, 5),
	/* 16 */ MuseumRow::new([0x01c, 0x021, 0x03c, 0x048, 0x101], 0x50214f, 1, 0),
	/* 17 */ MuseumRow::new([0x014, 0x02a, 0x03f, 0x042, 0x101], 0x502173, 1, 1),
	/* 18 */ MuseumRow::new([0x01a, 0x01e, 0x02f, 0x04c, 0x101], 0x502197, 1, 2),
	/* 19 */ MuseumRow::new([0x06a, 0x101, 0x101, 0x101, 0x101], 0x5021ad, 1, 3),
	/* 20 */ MuseumRow::new([0x04e, 0x052, 0x056, 0x057, 0x066], 0x502023, 0, 6),
	/* 21 */ MuseumRow::new([0x050, 0x051, 0x054, 0x058, 0x067], 0x50202b, 0, 7),
	/* 22 */ MuseumRow::new([0x04f, 0x053, 0x055, 0x05d, 0x05f], 0x50203c, 0, 9),
	/* 23 */ MuseumRow::new([0x013, 0x015, 0x016, 0x101, 0x101], 0x50d291, 0, 0),
	/* 24 */ MuseumRow::new([0x020, 0x029, 0x05b, 0x101, 0x101], 0x50d29b, 0, 0),
	/* 25 */ MuseumRow::new([0x02b, 0x030, 0x031, 0x101, 0x101], 0x50d265, 0, 0),
	/* 26 */ MuseumRow::new([0x033, 0x039, 0x03a, 0x101, 0x101], 0x50d270, 0, 0),
	/* 27 */ MuseumRow::new([0x03b, 0x03d, 0x03e, 0x101, 0x101], 0x50d27b, 0, 0),
	/* 28 */ MuseumRow::new([0x040, 0x041, 0x05c, 0x101, 0x101], 0x50d286, 0, 0),
];

#[repr(C)]
struct Color8 {
	r: u8,
	g: u8,
	b: u8,
	a: u8,
}

impl const From<[u8; 4]> for Color8 {
	fn from(array: [u8; 4]) -> Self {
		Self {
			r: array[0],
			g: array[1],
			b: array[2],
			a: array[3],
		}
	}
}

#[repr(C)]
struct MuseumRowColor {
	unk1: Color8,
	background: Color8,
	edge_fade: Color8,
	unk3: Color8,
}

impl MuseumRowColor {
	const fn new(
		background: impl ~const Into<Color8>,
		edge_fade: impl ~const Into<Color8>,
	) -> Self {
		// every row use these colors for unk1, and unk3 respectively
		let unk1 = Color8 {
			r: 0x00,
			g: 0x00,
			b: 0x00,
			a: 0x00,
		};
		let unk3 = Color8 {
			r: 0xff,
			g: 0xff,
			b: 0xff,
			a: 0x6e,
		};

		Self {
			background: background.into(),
			unk1,
			edge_fade: edge_fade.into(),
			unk3,
		}
	}
}

#[rustfmt::skip]
static MY_MUSEUM_ROW_COLORS: [MuseumRowColor; MUSEUM_ROW_COUNT] = [
	/* E2 */ MuseumRowColor::new([0x42, 0x42, 0x42, 0xFF], [0x00, 0x00, 0x00, 0x00]),
	/* E1 */ MuseumRowColor::new([0x42, 0x42, 0x42, 0xFF], [0x00, 0x00, 0x00, 0x00]),
	/* E0 */ MuseumRowColor::new([0x42, 0x42, 0x42, 0xFF], [0x00, 0x00, 0x00, 0x00]),

	/* 0  */ MuseumRowColor::new([0xFF, 0xED, 0x2A, 0xFF], [0x8C, 0x64, 0x00, 0x00]),
	/* 1  */ MuseumRowColor::new([0x4A, 0xF8, 0xF1, 0xFF], [0x01, 0x49, 0x4C, 0x00]),
	/* 2  */ MuseumRowColor::new([0xDC, 0xD9, 0xD9, 0xFF], [0x85, 0x56, 0x16, 0x00]),
	/* 3  */ MuseumRowColor::new([0xFF, 0xA0, 0x30, 0xFF], [0x78, 0x3A, 0x00, 0x00]),
	/* 4  */ MuseumRowColor::new([0x36, 0x67, 0xCA, 0xFF], [0x04, 0x00, 0x78, 0x00]),
	/* 5  */ MuseumRowColor::new([0xCE, 0xCA, 0xCA, 0xFF], [0x82, 0x39, 0x39, 0x00]),
	/* 6  */ MuseumRowColor::new([0xEB, 0x40, 0x40, 0xFF], [0x4F, 0x00, 0x00, 0x00]),
	/* 7  */ MuseumRowColor::new([0x9A, 0x43, 0xB7, 0xFF], [0x0F, 0x01, 0x3F, 0x00]),
	/* 8  */ MuseumRowColor::new([0xAF, 0xAF, 0xAF, 0xFF], [0x28, 0x4D, 0x2F, 0x00]),
	/* 9  */ MuseumRowColor::new([0x2B, 0xA8, 0x20, 0xFF], [0x01, 0x3F, 0x0D, 0x00]),
	/* 10 */ MuseumRowColor::new([0x2B, 0xA8, 0x20, 0xFF], [0x01, 0x3F, 0x0D, 0x00]),
	/* 11 */ MuseumRowColor::new([0x2B, 0xA8, 0x20, 0xFF], [0x01, 0x3F, 0x0D, 0x00]),
	/* 12 */ MuseumRowColor::new([0x2B, 0xA8, 0x20, 0xFF], [0x01, 0x3F, 0x0D, 0x00]),
	/* 13 */ MuseumRowColor::new([0x2B, 0xA8, 0x20, 0xFF], [0x01, 0x3F, 0x0D, 0x00]),
	/* 14 */ MuseumRowColor::new([0x2B, 0xA8, 0x20, 0xFF], [0x01, 0x3F, 0x0D, 0x00]),
	/* 15 */ MuseumRowColor::new([0x2B, 0xA8, 0x20, 0xFF], [0x01, 0x3F, 0x0D, 0x00]),
	/* 16 */ MuseumRowColor::new([0xFF, 0xCA, 0xED, 0xFF], [0x7E, 0x3E, 0x92, 0x00]),
	/* 17 */ MuseumRowColor::new([0xB7, 0xEC, 0xE2, 0xFF], [0x2E, 0x81, 0x86, 0x00]),
	/* 18 */ MuseumRowColor::new([0xEC, 0xE5, 0xB7, 0xFF], [0xAD, 0xAA, 0x08, 0x00]),
	/* 19 */ MuseumRowColor::new([0x8C, 0x8C, 0x8C, 0xFF], [0x50, 0x3C, 0x50, 0x00]),
	/* 20 */ MuseumRowColor::new([0xF7, 0xF2, 0xB9, 0xFF], [0xAA, 0x94, 0x3C, 0x00]),
	/* 21 */ MuseumRowColor::new([0xF7, 0xF2, 0xB9, 0xFF], [0xAA, 0x94, 0x3C, 0x00]),
	/* 22 */ MuseumRowColor::new([0xF7, 0xF2, 0xB9, 0xFF], [0xAA, 0x94, 0x3C, 0x00]),
	/* 23 */ MuseumRowColor::new([0x78, 0x50, 0x0A, 0xFF], [0x64, 0x3C, 0x32, 0x00]),
	/* 24 */ MuseumRowColor::new([0x78, 0x50, 0x0A, 0xFF], [0x64, 0x3C, 0x32, 0x00]),
	/* 25 */ MuseumRowColor::new([0x78, 0x50, 0x0A, 0xFF], [0x64, 0x3C, 0x32, 0x00]),
	/* 26 */ MuseumRowColor::new([0x78, 0x50, 0x0A, 0xFF], [0x64, 0x3C, 0x32, 0x00]),
	/* 27 */ MuseumRowColor::new([0x78, 0x50, 0x0A, 0xFF], [0x64, 0x3C, 0x32, 0x00]),
	/* 28 */ MuseumRowColor::new([0x78, 0x50, 0x0A, 0xFF], [0x64, 0x3C, 0x32, 0x00]),
];

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	// what happens if you panic in the panic handler :?
	todo!()
}
