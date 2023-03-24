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
		b_if_from 0x2424A8 {no_row_found_inject}
		mov pc, lr
	
	.pool
	"#,
	no_row_found_inject = sym no_row_found_inject,
);

const MUSEUM_ROW_COUNT: usize = 29;

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

fn no_row_found_inject() {
	// get the direction we we're searching in when no row was found
	let dir: SearchDirection = unsafe {
		let reg: u32;
		asm!("mov {}, r9", out(reg) reg);
		reg.try_into().unwrap()
	};

	let new_row_index = match dir {
		SearchDirection::Up => 0, // first row index

		SearchDirection::Down => (0..MUSEUM_ROW_COUNT)
			.rev()
			.find(|&index| row_is_visible(index))
			.unwrap_or(0),
	};

	unsafe { asm!("mov r0, {}", in(reg) new_row_index, out("r0") _) };
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
		// I don't know how to remove the ptr indirection
		// rust isn't happy with |const _:        $fptr_type = transmute( $addr)|
		// it only accepts       |const _: *const $fptr_type = transmute(&$addr)|
		const $name: *const $fptr_type = unsafe { core::mem::transmute(&$addr) };
	};
}

fn get_game_id(game_index: u16) -> u8 {
	const_fn_ptr_at_addr!(const FN_PTR at 0x261a10: extern "C" fn(u16) -> u8);
	unsafe { (*FN_PTR)(game_index) }
}

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
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

type SaveManager = ();
const SAVE_MANAGER: *const *const SaveManager = 0x54d350 as _;

fn get_game_rank(game_id: u8) -> GameRank {
	const_fn_ptr_at_addr! {
		const FN_PTR at 0x2619c4: extern "C" fn(
			save_manager: *const SaveManager,
			game_id: u8,
			save_slot: i32
		) -> GameRank
	};

	unsafe { (*FN_PTR)(*SAVE_MANAGER, game_id, -1) }
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

fn row_is_visible(row_index: usize) -> bool {
	let row = unsafe { &(*MUSEUM_ROWS)[row_index] };

	// I don't think this is possible
	// but the decompiled code does a similar check, so might as well
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
