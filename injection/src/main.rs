#![no_std]
#![no_main]

use core::{
	arch::{asm, global_asm},
	ffi::c_char,
};

global_asm!(
	r#"
	// redirects to `dest` function if _start was called from `addr`
	// the stack pointer is stored in r12 so the function can access it
	// without having to worry about what rust is doing with the stack
	.macro b_if_from addr, dest
		ldr r12, =(\addr + 4)
		cmp lr, r12
		mov r12, sp
		beq \dest
	.endm

	.global _start
	.section .text._start
	_start:
		b_if_from 0x2423DC {get_next_row}
		mov pc, lr
	
	.pool
	"#,
	get_next_row = sym get_next_row,
);

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

const MUSEUM_ROWS: *const [MuseumRow; MUSEUM_ROW_COUNT] = 0x4c8ea8 as _;

macro_rules! const_fn_ptr_at_addr {
	(const $name:ident at $addr:literal: $fptr_type:ty) => {
		const $name: *const $fptr_type = &$addr as *const _ as _;
	};
}

fn get_game_id(game_index: u16) -> u8 {
	const_fn_ptr_at_addr!(const FN_PTR at 0x261a10: extern "C" fn(u16) -> u8);
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
