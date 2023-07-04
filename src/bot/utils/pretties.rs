use std::{fmt, sync::OnceLock};

use is_prime::is_prime as check_prime;
use itertools::Itertools;
use pcre2::bytes::Regex;

#[derive(Clone, Copy, Debug)]
pub enum Effect {
	Complete,
	Palindrome,
	AllSameDigit,
	Sandwich,
	BracketingPair,
	TwoPairs,
	DecimalFullRound,
	DecimalPartRound,
	BinaryRound,
	Incrementing,
	Decrementing,
	Prime,
	Fibonacci,
	Weird,
	Untouchable,
	Square,
	Perfect,
}

impl Effect {
	pub fn all_from(n: u64) -> Vec<Self> {
		let mut all = Vec::new();

		if is_all_same_digit(n) {
			all.push(Self::AllSameDigit);
		}
		if is_sandwich(n) {
			all.push(Self::Sandwich);
		}

		if is_palindrome(n) {
			all.push(Self::Palindrome);
		} else if has_two_pairs(n) {
			all.push(Self::TwoPairs);
		} else if is_bracketing_pair(n) {
			all.push(Self::BracketingPair);
		}

		if is_decimal_full_round(n) {
			all.push(Self::DecimalFullRound);
		} else if is_decimal_part_round(n) {
			all.push(Self::DecimalPartRound);
		}

		if is_incrementing(n) {
			all.push(Self::Incrementing);
		} else if is_decrementing(n) {
			all.push(Self::Decrementing);
		}

		if is_binary_round(n) {
			all.push(Self::BinaryRound);
		}

		if is_prime(n) {
			all.push(Self::Prime);
		}
		if is_fibonacci(n) {
			all.push(Self::Fibonacci);
		}
		if is_weird(n) {
			all.push(Self::Weird);
		}
		if is_untouchable(n) {
			all.push(Self::Untouchable);
		}
		if is_square(n) {
			all.push(Self::Square);
		}
		if is_perfect(n) {
			all.push(Self::Perfect);
		}

		all
	}

	pub fn decorate(n: u64, is_complete: bool) -> (bool, String) {
		let mut pretties = Self::all_from(n);
		if pretties.is_empty() {
			return (false, n.to_string());
		}

		if is_complete {
			pretties.insert(0, Self::Complete);
		}

		(
			true,
			format!(
				"{fwd}{n}{rev}",
				fwd = pretties.iter().map(|e| e.to_string()).join(""),
				rev = pretties.into_iter().rev().map(|e| e.to_string()).join("")
			),
		)
	}

	pub fn on_after(mut n: u64) -> u64 {
		while Self::all_from(n + 1).is_empty() {
			n += 1;
		}

		n + 1
	}
}

impl fmt::Display for Effect {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Complete => '🎆',
				Self::Palindrome => '✨',
				Self::AllSameDigit => '🌉',
				Self::Sandwich => '🥪',
				Self::TwoPairs => '👀',
				Self::BracketingPair => '💞',
				Self::DecimalFullRound => '💫',
				Self::DecimalPartRound => '🌻',
				Self::Incrementing => '🌌',
				Self::Decrementing => '🌆',
				Self::BinaryRound => '🤖',
				Self::Prime => '🥇',
				Self::Fibonacci => '🤌',
				Self::Weird => '👾',
				Self::Untouchable => '🙅',
				Self::Square => '🆒',
				Self::Perfect => '💯',
			}
		)
	}
}

fn digits(mut n: u64) -> Vec<u8> {
	if n == 0 {
		return vec![0];
	}

	let mut digits = Vec::with_capacity(16);
	while n > 0 {
		digits.push((n % 10) as _);
		n /= 10;
	}
	digits.reverse();
	digits
}
#[test]
fn test_digits() {
	assert_eq!(digits(12345), vec![1, 2, 3, 4, 5]);
	assert_eq!(digits(827410), vec![8, 2, 7, 4, 1, 0]);
	assert_eq!(digits(6), vec![6]);
	assert_eq!(digits(0), vec![0]);
	assert_eq!(digits(100000), vec![1, 0, 0, 0, 0, 0]);
	assert_eq!(digits(10203040), vec![1, 0, 2, 0, 3, 0, 4, 0]);
}

fn bytes(n: u64) -> Vec<u8> {
	digits(n).into_iter().map(|n| n + 48).collect()
}

fn is_single_digit(n: u64) -> bool {
	n < 10
}
#[test]
fn test_is_single_digit() {
	assert!(is_single_digit(0));
	assert!(is_single_digit(1));
	assert!(is_single_digit(2));
	assert!(is_single_digit(3));
	assert!(is_single_digit(4));
	assert!(is_single_digit(5));
	assert!(is_single_digit(6));
	assert!(is_single_digit(7));
	assert!(is_single_digit(8));
	assert!(is_single_digit(9));
	assert!(!is_single_digit(10));
}

fn is_palindrome(n: u64) -> bool {
	if is_single_digit(n) {
		return false;
	}

	let fwd = digits(n);
	let mut rev = fwd.clone();
	rev.reverse();
	fwd == rev
}
#[test]
fn test_is_palindrome() {
	assert!(is_palindrome(121));
	assert!(is_palindrome(23810201832));
	assert!(is_palindrome(101010101));
	assert!(is_palindrome(8888));
	assert!(is_palindrome(1221));
	assert!(!is_palindrome(7));
	assert!(!is_palindrome(912));
	assert!(!is_palindrome(8212783));
	assert!(!is_palindrome(12345321));
	assert!(!is_palindrome(123432));
	assert!(!is_palindrome(10000000000));
}

pub fn palindrome_after(mut n: u64) -> u64 {
	while !is_palindrome(n + 1) {
		n += 1;
	}

	n + 1
}

fn is_all_same_digit(n: u64) -> bool {
	let mut dig = digits(n);
	dig.sort();
	dig.dedup();
	dig.len() == 1
}
#[test]
fn test_is_all_same_digit() {
	assert!(is_all_same_digit(0));
	assert!(is_all_same_digit(7));
	assert!(is_all_same_digit(333));
	assert!(!is_all_same_digit(82828));
	assert!(!is_all_same_digit(1239));
}

macro_rules! is_regex_match {
	($name:ident, $regex:expr) => {
		fn $name(n: u64) -> bool {
			#[allow(non_upper_case_globals)]
			static $name: OnceLock<Regex> = OnceLock::new();
			let rx = $name.get_or_init(|| Regex::new($regex).unwrap());
			rx.is_match(&bytes(n)).unwrap()
		}
	};
}

is_regex_match!(is_sandwich, r"^(\d)(\d)\2+\1$");
#[test]
fn test_is_sandwich() {
	assert!(is_sandwich(12221));
	assert!(is_sandwich(8338));
	assert!(is_sandwich(4000000004));
	assert!(!is_sandwich(123));
	assert!(!is_sandwich(9));
	assert!(!is_sandwich(0));
	assert!(!is_sandwich(10000));
	assert!(!is_sandwich(811121118));
}

is_regex_match!(has_two_pairs, r"(\d)\1.*(\d)\2");
#[test]
fn test_has_two_pairs() {
	assert!(has_two_pairs(1100011));
	assert!(has_two_pairs(2233));
	assert!(has_two_pairs(991231011));
	assert!(has_two_pairs(11223));
	assert!(has_two_pairs(29928123784117139));
	assert!(!has_two_pairs(0));
	assert!(!has_two_pairs(9));
	assert!(!has_two_pairs(2012));
	assert!(!has_two_pairs(12381));
	assert!(!has_two_pairs(292812378417139));
}

is_regex_match!(is_bracketing_pair, r"(\d{2}).*\1");
#[test]
fn test_is_bracketing_pair() {
	assert!(is_bracketing_pair(1212));
	assert!(is_bracketing_pair(9233392));
	assert!(is_bracketing_pair(55155));
	assert!(is_bracketing_pair(7777));
	assert!(!is_bracketing_pair(1234));
	assert!(!is_bracketing_pair(6));
	assert!(!is_bracketing_pair(12344));
	assert!(!is_bracketing_pair(9980));
}

is_regex_match!(is_decimal_full_round, r"^\d0+$");
#[test]
fn test_is_decimal_full_round() {
	assert!(is_decimal_full_round(10));
	assert!(is_decimal_full_round(200));
	assert!(is_decimal_full_round(9000));
	assert!(!is_decimal_full_round(120000));
	assert!(!is_decimal_full_round(123456789));
}

is_regex_match!(is_decimal_part_round, r"^\d+0{2,}$");
#[test]
fn test_is_decimal_part_round() {
	assert!(is_decimal_part_round(200));
	assert!(is_decimal_part_round(9000));
	assert!(is_decimal_part_round(120000));
	assert!(!is_decimal_part_round(10));
	assert!(!is_decimal_part_round(123456789));
	assert!(!is_decimal_part_round(82));
	assert!(!is_decimal_part_round(70002));
}

fn is_binary_round(n: u64) -> bool {
	if n == 0 {
		return false;
	}

	n.is_power_of_two()
}
#[test]
fn test_is_binary_round() {
	assert!(is_binary_round(8));
	assert!(is_binary_round(16));
	assert!(is_binary_round(32));
	assert!(is_binary_round(1024));
	assert!(is_binary_round(33554432));
	assert!(!is_binary_round(0));
	assert!(!is_binary_round(123));
	assert!(!is_binary_round(2012783));
	assert!(!is_binary_round(255));
	assert!(!is_binary_round(12224));
}

fn is_incrementing(n: u64) -> bool {
	if is_single_digit(n) {
		return false;
	}

	let digs = digits(n);
	let mut sort = digs.clone();
	sort.sort();
	sort == digs
}
#[test]
fn test_is_incrementing() {
	assert!(!is_incrementing(1));
	assert!(is_incrementing(12));
	assert!(is_incrementing(234));
	assert!(is_incrementing(56789));
	assert!(!is_incrementing(321));
	assert!(!is_incrementing(8183901));
}

fn is_decrementing(n: u64) -> bool {
	if is_single_digit(n) {
		return false;
	}

	let digs = digits(n);
	let mut sort = digs.clone();
	sort.sort();
	sort.reverse();
	sort == digs
}
#[test]
fn test_is_decrementing() {
	assert!(!is_decrementing(1));
	assert!(is_decrementing(21));
	assert!(is_decrementing(432));
	assert!(is_decrementing(98765));
	assert!(!is_decrementing(123));
	assert!(!is_decrementing(8183901));
}

fn is_prime(n: u64) -> bool {
	check_prime(&n.to_string())
}

fn is_fibonacci(n: u64) -> bool {
	// https://en.wikipedia.org/wiki/Fibonacci_number#Identification
	is_square(5 * n.pow(2) + 4) || is_square(5 * n.pow(2) - 4)
}
#[test]
fn test_is_fibonacci() {
	assert!(is_fibonacci(1));
	assert!(is_fibonacci(2));
	assert!(is_fibonacci(3));
	assert!(is_fibonacci(5));
	assert!(is_fibonacci(8));
	assert!(is_fibonacci(24157817));
	assert!(!is_fibonacci(1827382));
	assert!(!is_fibonacci(4));
}

fn is_weird(n: u64) -> bool {
	// https://oeis.org/A006037
	// TODO: find more
	[
		70, 836, 4030, 5830, 7192, 7912, 9272, 10430, 10570, 10792, 10990, 11410, 11690, 12110,
		12530, 12670, 13370, 13510, 13790, 13930, 14770, 15610, 15890, 16030, 16310, 16730, 16870,
		17272, 17570, 17990, 18410, 18830, 18970, 19390, 19670,
	]
	.contains(&n)
}

fn is_untouchable(n: u64) -> bool {
	// https://oeis.org/A005114
	// TODO: find more
	[
		2, 5, 52, 88, 96, 120, 124, 146, 162, 188, 206, 210, 216, 238, 246, 248, 262, 268, 276,
		288, 290, 292, 304, 306, 322, 324, 326, 336, 342, 372, 406, 408, 426, 430, 448, 472, 474,
		498, 516, 518, 520, 530, 540, 552, 556, 562, 576, 584, 612, 624, 626, 628, 658,
	]
	.contains(&n)
}

fn is_square(n: u64) -> bool {
	let f = n as f64;
	f.sqrt().floor().powi(2) == f
}
#[test]
fn test_is_square() {
	assert!(is_square(1));
	assert!(is_square(4));
	assert!(is_square(16));
	assert!(is_square(9));
	assert!(is_square(81));
	assert!(is_square(10000));
	assert!(!is_square(2));
	assert!(!is_square(23));
	assert!(!is_square(18201));
	assert!(!is_square(10));
}

fn is_perfect(n: u64) -> bool {
	// https://oeis.org/A000396
	[6, 28, 496, 8128].contains(&n)
}
