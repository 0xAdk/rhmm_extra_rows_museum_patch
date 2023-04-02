#![no_std]
#![no_main]

use core::{arch::asm, ffi::c_char};

#[export_name = "_start"]
#[link_section = ".text._start"]
fn setup_hooks() {
	thunk_to_rust(0x2423D8 as _, get_next_row as _);
}

// takes a address to a function in code.bin and replaces it with a thunk
// function that calls `rust_function` instead
fn thunk_to_rust(func_addr: *const (), rust_function: *const ()) {
	// overwrites the first 3 bytes of func_addr with:
	//   ldr r12, 0f
	//   bx  r12
	//   0: .word {rust_function_ptr}
	let thunk_to_rust_func = &[0xE5_9F_C0_00, 0xE1_2F_FF_1C, rust_function as _];

	unsafe { patch_text(func_addr as _, thunk_to_rust_func).unwrap() }
}

unsafe fn patch_text(addr: *mut u32, new_code: &[u32]) -> Result<(), SvcResult> {
	let text_start = 0x100000 as *const ();
	let text_size = 0x29A000;

	let process_handle_wrapper = open_current_process_handle().unwrap();

	process_memory_set_permissions(
		process_handle_wrapper.handle,
		text_start,
		text_size,
		MemoryPermission::RW,
	)
	.unwrap();

	addr.copy_from_nonoverlapping(new_code.as_ptr(), new_code.len());

	process_memory_set_permissions(
		process_handle_wrapper.handle,
		text_start,
		text_size,
		MemoryPermission::RX,
	)
	.unwrap();

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

const MUSEUM_ROW_COUNT: usize = 29;

#[repr(u8)]
enum SearchDirection {
	Up = 0,
	Down = 1,
}

impl TryFrom<u32> for SearchDirection {
	type Error = ();

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(SearchDirection::Up),
			1 => Ok(SearchDirection::Down),
			_ => Err(()),
		}
	}
}

extern "C" fn get_next_row(_this: *const (), mut current_row: usize, dir: SearchDirection) -> i32 {
	// this shouldn't happen right?
	if current_row > MUSEUM_ROW_COUNT {
		return -1;
	}

	let new_row_index = loop {
		match dir {
			SearchDirection::Up => current_row += 1,
			SearchDirection::Down => current_row = current_row.wrapping_sub(1),
		}

		if current_row >= MUSEUM_ROW_COUNT {
			break None;
		}

		let row = unsafe { &(*MUSEUM_ROWS)[current_row] };
		if row_is_visible(row) {
			break Some(current_row);
		}
	};

	let current_save_slot = unsafe { (**SAVE_MANAGER).current_save_slot };

	match new_row_index {
		Some(row) => row as i32,
		None => match dir {
			SearchDirection::Up => 0,
			SearchDirection::Down => (0..MUSEUM_ROW_COUNT)
				.rev()
				.find(|&index| {
					let row = unsafe { &(*MUSEUM_ROWS)[index] };
					row_is_visible(row)
				})
				.unwrap_or(0) as i32,
		},
	}
}

#[repr(C)]
struct MuseumRow {
	column_count: usize,
	game_indices: [u16; 5],
	undefined: [u8; 2],
	title_id: *const c_char,
	high_index: u32,
	low_index: u32,
}

const MUSEUM_ROWS: *const [MuseumRow; MUSEUM_ROW_COUNT] = 0x4C8EA8 as _;

macro_rules! const_fn_ptr_at_addr {
	(const $name:ident at $addr:literal: $fptr_type:ty) => {
		const $name: *const $fptr_type = &$addr as *const _ as _;
	};
}

fn get_game_id(game_index: u16) -> u8 {
	const_fn_ptr_at_addr!(const FN_PTR at 0x261A10: extern "C" fn(u16) -> u8);
	unsafe { (*FN_PTR)(game_index) }
}

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum GameRank {
	Unknown = 0, // game is hidden in the campaign or not bought

	// the only difference is a shop game is shown in the museum
	// before it's been finished while a story game shows up as ???
	UnfinishedStory = 1,
	UnfinishedShop = 2,

	NotGood = 3, //       high score < 60
	Ok = 4,      // 60 <= high score < 80
	High = 5,    // 80 <= high_score
	Perfect = 6, // got a perfect in the perfect challenge
}

impl TryFrom<u8> for GameRank {
	type Error = ();

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		Ok(match value {
			0 => GameRank::Unknown,
			1 => GameRank::UnfinishedStory,
			2 => GameRank::UnfinishedShop,
			3 => GameRank::NotGood,
			4 => GameRank::Ok,
			5 => GameRank::High,
			6 => GameRank::Perfect,
			_ => return Err(()),
		})
	}
}

#[repr(C)]
struct SaveSlot {
	fill_0: [u8; 0x74],
	game_ranks: [GameRank; 104],
	fill_1: [u8; 0x10A8],
	coin_count: u16,
	flow_ball_count: u16,
	fill_2: [u8; 0x4C0],
}

#[repr(C)]
struct SaveManager {
	fill_0: [u8; 0x1C3F],
	save_slots: [SaveSlot; 4],
	current_save_slot: usize,
	fill_1: [u8; 0x4],
}

const SAVE_MANAGER: *const *const SaveManager = 0x54d350 as _;

fn get_game_rank(game_id: u8) -> GameRank {
	let save_manager = unsafe { &**SAVE_MANAGER };
	let current_save_slot = &save_manager.save_slots[save_manager.current_save_slot];
	current_save_slot.game_ranks[game_id as usize]
}

fn find_row_with_index(game_index: u16) -> u8 {
	const_fn_ptr_at_addr! {
		const FN_PTR at 0x261964: extern "C" fn(
			game_index: u16,
		) -> u8
	};

	unsafe { (*FN_PTR)(game_index) }
}

fn get_gate_state(high_index: u32, low_index: u32) -> u8 {
	const_fn_ptr_at_addr! {
		const FN_PTR at 0x261914: extern "C" fn(
			save_manager: *const SaveManager,
			high_index: u32,
			low_index: u32,
			save_slot: i32,
		) -> u8
	};

	unsafe { (*FN_PTR)(*SAVE_MANAGER, high_index, low_index, -1) }
}

fn row_is_visible(row: &MuseumRow) -> bool {
	// I don't think this is possible but the decompiled code does a similar
	// check, so might as well
	if row.column_count == 0 {
		return false;
	}

	row.game_indices[..row.column_count]
		.iter()
		.any(|&game_index| game_is_visible(game_index))
}

/// checks if a game is visible in the museum.
///
/// this is unrelated to the main campaign
// since a game is only visible (i.e. not show as ???) in the museum after
// you've completed it at least once
fn game_is_visible(game_index: u16) -> bool {
	let game_is_gate = game_index >= 0x68;

	if game_is_gate {
		let row_index = find_row_with_index(game_index) as usize;
		let row = unsafe { &(*MUSEUM_ROWS)[row_index] };

		let state = get_gate_state(row.high_index, row.low_index);

		// TODO: figure out what the values of the gate state enum mean
		state > 4
	} else {
		let game_id = get_game_id(game_index);
		get_game_rank(game_id) >= GameRank::UnfinishedShop
	}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
	// what happens if you panic in the panic handler :?
	todo!()
}
