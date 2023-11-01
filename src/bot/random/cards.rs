use std::{collections::HashSet, str::FromStr};

use miette::{miette, Report, Result};
use rand::seq::IteratorRandom;

pub const SUIT_ENGLISH: [&str; 4] = ["Clubs", "Diamonds", "Hearts", "Spades"];
pub const SUIT_FRENCH: [&str; 4] = ["Clovers", "Tiles", "Hearts", "Pikes"];
pub const SUIT_GERMAN: [&str; 4] = ["Hearts", "Bells", "Acorns", "Leaves"];
pub const SUIT_ITALIAN: [&str; 4] = ["Cups", "Coins", "Clubs", "Swords"];
pub const SUIT_SPANISH: [&str; 4] = SUIT_ITALIAN;
pub const SUIT_SWISS: [&str; 4] = ["Roses", "Bells", "Acorns", "Shields"];
pub const SUIT_TAROT: [&str; 4] = ["Cups", "Coins", "Batons", "Swords"];
pub const SUIT_NOUVEAU: [&str; 4] = ["Clovers", "Tiles", "Hearts", "Pikes"];
pub const SUIT_GANJIFA: [&str; 10] = [
	"Matsya",
	"Kurma",
	"Varaha",
	"Narasimha",
	"Vamana",
	"Parashurama",
	"Rama",
	"Krishna",
	"Buddha",
	"Kalki",
];
pub const SUIT_MOGHUL: [&str; 8] = [
	"Slaves غلام",
	"Crowns تاج",
	"Swords شمشير",
	"Red gold زر سرخ",
	"Harps چنگ",
	"Bills برات",
	"White gold زر سفيد",
	"Cloth قماش",
];
pub const SUIT_HANAFUDA: [&str; 15] = [
	"Pine",
	"Plum blossom",
	"Cherry blossom",
	"Wisteria",
	"Iris",
	"Peony",
	"Bush clover",
	"Susuki grass",
	"Chrysanthemum",
	"Maple",
	"Willow",
	"Paulownia",
	"Snow",
	"Earth",
	"Heaven",
];
pub const SUIT_MAHJONG: [&str; 3] = ["Circles", "Bamboos", "Characters"];

pub const PIPS_ONE_TEN: [&str; 10] = [
	"one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];
pub const PIPS_ONE_NINE: [&str; 9] = [
	"one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
];
pub const PIPS_ACE_TEN: [&str; 10] = [
	"ace", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];
pub const PIPS_ACE_THIRTEEN: [&str; 13] = [
	"ace", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
	"twelve", "thirteen",
];
pub const PIPS_ACE_TWELVE: [&str; 12] = [
	"ace", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
	"twelve",
];

pub const HONOURS_ONE_TWENTYONE: [&str; 21] = [
	"one",
	"two",
	"three",
	"four",
	"five",
	"six",
	"seven",
	"eight",
	"nine",
	"ten",
	"eleven",
	"twelve",
	"thirteen",
	"fourteen",
	"fifteen",
	"sixteen",
	"seventeen",
	"eighteen",
	"nineteen",
	"twenty",
	"twenty-one",
];

pub const FACES_STANDARD: [&str; 3] = ["Jack", "Queen", "King"];
pub const FACES_TAROT: [&str; 5] = ["Jack", "Page", "Knight", "Queen", "King"];
pub const FACES_NOUVEAU: [&str; 4] = ["Jack", "Knight", "Queen", "King"];
pub const FACES_SICILIANO: [&str; 4] = ["Maids", "Knight", "Queen", "King"];
pub const FACES_MINCHIATE: [&str; 5] = ["Maids", "Pages", "Knight", "Queen", "King"];
pub const FACES_BOLOGNESE: [&str; 4] = ["Knave", "Knight", "Queen", "King"];
pub const FACES_GANJIFA: [&str; 2] = ["Vizier", "King"];

pub const FOOL: &str = "Fool";
pub const JOKER: &str = "Joker";

pub const ASPECTS_ARCANA: [&str; 22] = [
	FOOL,
	"Magician",
	"High Priestess",
	"Empress",
	"Emperor",
	"Hierophant",
	"Lovers",
	"Chariot",
	"Strength",
	"Hermit",
	"Wheel of Fortune",
	"Justice",
	"Hanged Man",
	"Death",
	"Temperance",
	"Devil",
	"Tower",
	"Star",
	"Moon",
	"Sun",
	"Judgement",
	"World",
];

pub const ASPECTS_NOUVEAU: [&str; 24] = [
	"Excuse",
	"Individual 🃡",
	"Childhood 🃢",
	"Youth 🃣",
	"Maturity 🃤",
	"Old Age 🃥",
	"Morning 🃦",
	"Afternoon 🃧",
	"Evening 🃨",
	"Night 🃩",
	"Earth 🃪",
	"Air 🃪",
	"Water 🃫",
	"Fire 🃫",
	"Dance 🃬",
	"Shopping 🃭",
	"Open air 🃮",
	"Visual arts 🃯",
	"Spring 🃰",
	"Summer 🃱",
	"Autumn 🃲",
	"Winter 🃳",
	"The game 🃴",
	"Collective 🃵",
];

pub const ASPECTS_SICILIANO: [&str; 22] = [
	"Fugitive",
	"Miseria",
	"Mountebank",
	"Empress",
	"Emperor",
	"Constancy",
	"Temperance",
	"Fortitude",
	"Justice",
	"Love",
	"Chariot",
	"Wheel",
	"Hanged Man",
	"Time",
	"Death",
	"Ship",
	"Tower",
	"Star",
	"Moon",
	"Sun",
	"Atlas",
	"Jupiter",
];

pub const ASPECTS_MINCHIATE: [&str; 41] = [
	"Madman",
	"Uno",
	"Empress",
	"Emperor",
	"Pope",
	"Love",
	"Temperance",
	"Fortitude",
	"Justice",
	"Wheel of Fortune",
	"Chariot",
	"Time",
	"Traitor",
	"Death",
	"Devil",
	"House of the Devil",
	"Hope",
	"Prudence",
	"Faith",
	"Charity",
	"Fire",
	"Water",
	"Earth",
	"Air",
	"Libra",
	"Virgo",
	"Scorpio",
	"Aries",
	"Capricorn",
	"Sagittarius",
	"Cancer",
	"Pisces",
	"Aquarius",
	"Leo",
	"Taurus",
	"Gemini",
	"Star",
	"Moon",
	"Sun",
	"World",
	"Trumpets",
];

pub const ASPECTS_BOLOGNESE: [&str; 21] = [
	"Magician",
	"Moor",
	"Moor",
	"Moor",
	"Moor",
	"Love",
	"Chariot",
	"Temperance",
	"Justice",
	"Strength",
	"Wheel",
	"Old man",
	"Traitor",
	"Death",
	"Devil",
	"Lightning",
	"Star",
	"Moon",
	"Sun",
	"World",
	"Angel",
];

pub const ASPECTS_SWISS: [&str; 20] = [
	FOOL,
	"Magician",
	"Empress",
	"Emperor",
	"Lovers",
	"Chariot",
	"Justice",
	"Hermit",
	"Wheel of Fortune",
	"Strength",
	"Hanged Man",
	"Death",
	"Temperance",
	"Devil",
	"Tower",
	"Star",
	"Moon",
	"Sun",
	"Judgment",
	"World",
];

pub const VALUES_HANAFUDA: [&str; 4] = ["Hikari 光", "Tane 種", "Tanzaku 短冊", "Kasu カス"];

pub const MAHJONG_WINDS: [&str; 4] = ["East", "South", "West", "North"];
pub const MAHJONG_DRAGONS: [&str; 3] = ["Red", "Green", "White"];
pub const MAHJONG_SEASONS: [&str; 4] = ["Spring", "Summer", "Autumn", "Winter"];
pub const MAHJONG_FLOWERS: [&str; 4] = ["Plum", "Orchid", "Chrysanthemum", "Bamboo"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuitVariant {
	All,
	English,
	French,
	German,
	Italian,
	Spanish,
	Swiss,
	Tarot,
	Nouveau,
	Ganjifa,
	Moghul,
	Hanafuda,
	Mahjong,
}

impl FromStr for SuitVariant {
	type Err = Report;

	fn from_str(s: &str) -> Result<Self> {
		Ok(match s {
			"all" => Self::All,
			"english" => Self::English,
			"french" => Self::French,
			"german" => Self::German,
			"italian" => Self::Italian,
			"spanish" => Self::Spanish,
			"swiss" => Self::Swiss,
			"tarot" => Self::Tarot,
			"nouveau" => Self::Nouveau,
			"ganjifa" => Self::Ganjifa,
			"moghul" => Self::Moghul,
			"hanafuda" => Self::Hanafuda,
			"mahjong" => Self::Mahjong,
			_ => return Err(miette!("invalid suit variant")),
		})
	}
}

impl SuitVariant {
	pub fn random(self) -> String {
		let set = match self {
			SuitVariant::All => vec![
				&SUIT_ENGLISH[..],
				&SUIT_FRENCH[..],
				&SUIT_GERMAN[..],
				&SUIT_ITALIAN[..],
				&SUIT_SPANISH[..],
				&SUIT_SWISS[..],
				&SUIT_TAROT[..],
				&SUIT_NOUVEAU[..],
				&SUIT_GANJIFA[..],
				&SUIT_MOGHUL[..],
				&SUIT_HANAFUDA[..],
				&SUIT_MAHJONG[..],
			],
			SuitVariant::English => vec![&SUIT_ENGLISH[..]],
			SuitVariant::French => vec![&SUIT_FRENCH[..]],
			SuitVariant::German => vec![&SUIT_GERMAN[..]],
			SuitVariant::Italian => vec![&SUIT_ITALIAN[..]],
			SuitVariant::Spanish => vec![&SUIT_SPANISH[..]],
			SuitVariant::Swiss => vec![&SUIT_SWISS[..]],
			SuitVariant::Tarot => vec![&SUIT_TAROT[..]],
			SuitVariant::Nouveau => vec![&SUIT_NOUVEAU[..]],
			SuitVariant::Ganjifa => vec![&SUIT_GANJIFA[..]],
			SuitVariant::Moghul => vec![&SUIT_MOGHUL[..]],
			SuitVariant::Hanafuda => vec![&SUIT_HANAFUDA[..]],
			SuitVariant::Mahjong => vec![&SUIT_MAHJONG[..]],
		}
		.into_iter()
		.flat_map(|s| s.iter())
		.collect::<HashSet<&&str>>();

		set.into_iter()
			.choose(&mut rand::thread_rng())
			.unwrap()
			.to_string()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueVariant {
	All,
	Full,
	Euchre,
	Tarot,
	Arcana,
	Honours,
	Nouveau,
	Aspects,
	SicilianoValues,
	SicilianoHonours,
	BologneseValues,
	BologneseHonours,
	MinchiateValues,
	MinchiateHonours,
	OneJJ,
	Ganjifa,
	Hanafuda,
	Mahjong,
}

impl FromStr for ValueVariant {
	type Err = Report;

	fn from_str(s: &str) -> Result<Self> {
		Ok(match s {
			"all" => Self::All,
			"full" => Self::Full,
			"euchre" => Self::Euchre,
			"tarot" => Self::Tarot,
			"arcana" => Self::Arcana,
			"honours" => Self::Honours,
			"nouveau" => Self::Nouveau,
			"aspects" => Self::Aspects,
			"siciliano-v" => Self::SicilianoValues,
			"siciliano-h" => Self::SicilianoHonours,
			"bolognese-v" => Self::BologneseValues,
			"bolognese-h" => Self::BologneseHonours,
			"minchiate-v" => Self::MinchiateValues,
			"minchiate-h" => Self::MinchiateHonours,
			"1jj" => Self::OneJJ,
			"ganjifa" => Self::Ganjifa,
			"hanafuda" => Self::Hanafuda,
			"mahjong" => Self::Mahjong,
			_ => return Err(miette!("invalid value variant")),
		})
	}
}

impl ValueVariant {
	pub fn set(&self) -> HashSet<&&str> {
		match self {
			ValueVariant::All => vec![
				&PIPS_ACE_TEN[..],
				&PIPS_ACE_THIRTEEN[..],
				&PIPS_ONE_TEN[..],
				&PIPS_ONE_NINE[..],
				&[JOKER, FOOL],
				&FACES_STANDARD[..],
				&FACES_TAROT[..],
				&FACES_NOUVEAU[..],
				&FACES_SICILIANO[..],
				&FACES_BOLOGNESE[..],
				&FACES_MINCHIATE[..],
				&FACES_GANJIFA[..],
				&ASPECTS_ARCANA[..],
				&ASPECTS_NOUVEAU[..],
				&ASPECTS_SICILIANO[..],
				&ASPECTS_BOLOGNESE[..],
				&ASPECTS_MINCHIATE[..],
				&ASPECTS_SWISS[..],
				&VALUES_HANAFUDA[..],
				&MAHJONG_WINDS[..],
				&MAHJONG_DRAGONS[..],
				&MAHJONG_SEASONS[..],
				&MAHJONG_FLOWERS[..],
			],
			ValueVariant::Full => vec![&PIPS_ACE_TEN[..], &[JOKER, JOKER], &FACES_STANDARD[..]],
			ValueVariant::Euchre => vec![&PIPS_ACE_THIRTEEN[..], &[JOKER], &FACES_STANDARD[..]],
			ValueVariant::Tarot => vec![&PIPS_ONE_TEN[..], &FACES_TAROT[..]],
			ValueVariant::Arcana => vec![&ASPECTS_ARCANA[..]],
			ValueVariant::Nouveau => vec![&PIPS_ONE_TEN[..], &FACES_NOUVEAU[..]],
			ValueVariant::Honours => vec![&HONOURS_ONE_TWENTYONE[..], &[FOOL]],
			ValueVariant::Aspects => vec![&ASPECTS_NOUVEAU[..]],
			ValueVariant::SicilianoValues => vec![&PIPS_ONE_TEN[..], &FACES_SICILIANO[..]],
			ValueVariant::SicilianoHonours => vec![&ASPECTS_SICILIANO[..]],
			ValueVariant::BologneseValues => vec![&PIPS_ONE_TEN[..], &FACES_BOLOGNESE[..]],
			ValueVariant::BologneseHonours => vec![&ASPECTS_BOLOGNESE[..]],
			ValueVariant::MinchiateValues => vec![&PIPS_ONE_TEN[..], &FACES_MINCHIATE[..]],
			ValueVariant::MinchiateHonours => vec![&ASPECTS_MINCHIATE[..]],
			ValueVariant::OneJJ => vec![&ASPECTS_SWISS[..]],
			ValueVariant::Ganjifa => vec![&PIPS_ONE_TEN[..], &FACES_GANJIFA[..]],
			ValueVariant::Hanafuda => vec![&VALUES_HANAFUDA[..]],
			ValueVariant::Mahjong => vec![
				&PIPS_ONE_NINE[..],
				&MAHJONG_WINDS[..],
				&MAHJONG_DRAGONS[..],
				&MAHJONG_SEASONS[..],
				&MAHJONG_FLOWERS[..],
			],
		}
		.into_iter()
		.flat_map(|s| s.iter())
		.collect()
	}

	pub fn random(self) -> String {
		self.set()
			.into_iter()
			.choose(&mut rand::thread_rng())
			.unwrap()
			.to_string()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckVariant {
	All,
	English,
	French,
	German,
	Italian,
	Spanish,
	Swiss,
	Euchre,
	Tarot,
	Nouveau,
	Siciliano,
	Bolognese,
	Minchiate,
	OneJJ,
	Ganjifa,
	Moghul,
	Hanafuda,
	Mahjong,
}

impl FromStr for DeckVariant {
	type Err = Report;

	fn from_str(s: &str) -> Result<Self> {
		Ok(match s {
			"all" => Self::All,
			"english" => Self::English,
			"french" => Self::French,
			"german" => Self::German,
			"italian" => Self::Italian,
			"spanish" => Self::Spanish,
			"swiss" => Self::Swiss,
			"euchre" => Self::Euchre,
			"tarot" => Self::Tarot,
			"nouveau" => Self::Nouveau,
			"siciliano" => Self::Siciliano,
			"bolognese" => Self::Bolognese,
			"minchiate" => Self::Minchiate,
			"1jj" => Self::OneJJ,
			"ganjifa" => Self::Ganjifa,
			"moghul" => Self::Moghul,
			"hanafuda" => Self::Hanafuda,
			"mahjong" => Self::Mahjong,
			_ => return Err(miette!("invalid deck variant")),
		})
	}
}

impl DeckVariant {
	fn set(&self) -> HashSet<String> {
		match self {
			DeckVariant::All => {
				let mut superset = HashSet::new();
				for variant in &[
					DeckVariant::English,
					DeckVariant::French,
					DeckVariant::German,
					DeckVariant::Italian,
					DeckVariant::Spanish,
					DeckVariant::Swiss,
					DeckVariant::Euchre,
					DeckVariant::Tarot,
					DeckVariant::Nouveau,
					DeckVariant::Siciliano,
					DeckVariant::Bolognese,
					DeckVariant::Minchiate,
					DeckVariant::OneJJ,
					DeckVariant::Ganjifa,
					DeckVariant::Moghul,
					DeckVariant::Hanafuda,
					DeckVariant::Mahjong,
				] {
					superset.extend(variant.set());
				}
				return superset;
			}
			DeckVariant::English => vec![
				vec![JOKER.to_string()],
				build_suits(
					&SUIT_ENGLISH,
					vec![&PIPS_ACE_TEN[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
			],
			DeckVariant::French => vec![
				vec![JOKER.to_string()],
				build_suits(
					&SUIT_ENGLISH,
					vec![&PIPS_ACE_TEN[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
			],
			DeckVariant::German => vec![
				vec![JOKER.to_string()],
				build_suits(
					&SUIT_GERMAN,
					vec![&PIPS_ACE_TEN[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
			],
			DeckVariant::Italian => vec![
				vec![JOKER.to_string()],
				build_suits(
					&SUIT_ITALIAN,
					vec![&PIPS_ACE_TEN[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
			],
			DeckVariant::Spanish => vec![
				vec![JOKER.to_string()],
				build_suits(
					&SUIT_SPANISH,
					vec![&PIPS_ACE_TEN[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
			],
			DeckVariant::Swiss => vec![
				vec![JOKER.to_string()],
				build_suits(
					&SUIT_SWISS,
					vec![&PIPS_ACE_TEN[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
			],
			DeckVariant::Euchre => vec![
				vec![JOKER.to_string()],
				build_suits(
					&[SUIT_FRENCH[1], SUIT_FRENCH[2]], // red
					vec![&PIPS_ACE_THIRTEEN[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
				build_suits(
					&[SUIT_FRENCH[0], SUIT_FRENCH[3]], // black
					vec![&PIPS_ACE_TWELVE[..], &FACES_STANDARD[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
			],
			DeckVariant::Tarot => vec![
				build_suits(
					&SUIT_TAROT,
					vec![&PIPS_ONE_TEN[..], &FACES_TAROT[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
				ASPECTS_ARCANA.iter().map(|s| s.to_string()).collect(),
			],
			DeckVariant::Nouveau => vec![
				build_suits(
					&SUIT_NOUVEAU,
					vec![&PIPS_ONE_TEN[..], &FACES_NOUVEAU[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
				ASPECTS_NOUVEAU.iter().map(|s| s.to_string()).collect(),
			],
			DeckVariant::Siciliano => vec![
				build_suits(
					&SUIT_ITALIAN,
					vec![&PIPS_ONE_TEN[..], &FACES_SICILIANO[..]]
						.into_iter()
						.flat_map(|s| s.iter()),
				),
				ASPECTS_SICILIANO.iter().map(|s| s.to_string()).collect(),
			],
			DeckVariant::Bolognese => vec![
				build_suits(
					&SUIT_ITALIAN,
					vec![
						&["ace", "six", "seven", "eight", "nine", "ten"][..],
						&FACES_BOLOGNESE[..],
					]
					.into_iter()
					.flat_map(|s| s.iter()),
				),
				ASPECTS_BOLOGNESE.iter().map(|s| s.to_string()).collect(),
			],
			DeckVariant::Minchiate => vec![
				build_suits(
					&[SUIT_ITALIAN[0], SUIT_ITALIAN[1]], // cups and coins
					vec![
						&["ace", "six", "seven", "eight", "nine", "ten"][..],
						&[
							FACES_MINCHIATE[0], // maids
							FACES_MINCHIATE[2],
							FACES_MINCHIATE[3],
							FACES_MINCHIATE[4],
						],
					]
					.into_iter()
					.flat_map(|s| s.iter()),
				),
				build_suits(
					&[SUIT_ITALIAN[2], SUIT_ITALIAN[3]], // swords and clubs
					vec![
						&["ace", "six", "seven", "eight", "nine", "ten"][..],
						&[
							FACES_MINCHIATE[1], // pages
							FACES_MINCHIATE[2],
							FACES_MINCHIATE[3],
							FACES_MINCHIATE[4],
						],
					]
					.into_iter()
					.flat_map(|s| s.iter()),
				),
				ASPECTS_MINCHIATE.iter().map(|s| s.to_string()).collect(),
			],
			DeckVariant::OneJJ => vec![
				build_suits(
					&SUIT_ITALIAN,
					vec![
						&PIPS_ONE_TEN[..],
						&[
							FACES_TAROT[1], // pages
							FACES_TAROT[2],
							FACES_TAROT[3],
							FACES_TAROT[4],
						],
					]
					.into_iter()
					.flat_map(|s| s.iter()),
				),
				ASPECTS_SWISS.iter().map(|s| s.to_string()).collect(),
			],
			DeckVariant::Ganjifa => vec![build_suits(
				&SUIT_GANJIFA,
				vec![&PIPS_ONE_TEN[..], &FACES_GANJIFA[..]]
					.into_iter()
					.flat_map(|s| s.iter()),
			)],
			DeckVariant::Moghul => vec![build_suits(
				&SUIT_MOGHUL,
				vec![&PIPS_ONE_TEN[..], &FACES_GANJIFA[..]]
					.into_iter()
					.flat_map(|s| s.iter()),
			)],
			DeckVariant::Hanafuda => vec![vec![
				// january
				format!("{} {}: Crane and sun", SUIT_HANAFUDA[0], VALUES_HANAFUDA[0]),
				format!("{} {}", SUIT_HANAFUDA[0], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[0], VALUES_HANAFUDA[3]), // x2
				// february
				format!("{} {}: Bush warbler", SUIT_HANAFUDA[1], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[1], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[1], VALUES_HANAFUDA[3]), // x2
				// march
				format!("{} {}: Curtain", SUIT_HANAFUDA[2], VALUES_HANAFUDA[0]),
				format!("{} {}", SUIT_HANAFUDA[2], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[2], VALUES_HANAFUDA[3]), // x2
				// april
				format!("{} {}: Cuckoo", SUIT_HANAFUDA[3], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[3], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[3], VALUES_HANAFUDA[3]), // x2
				// may
				format!(
					"{} {}: Eight-plank bridge",
					SUIT_HANAFUDA[4], VALUES_HANAFUDA[1]
				),
				format!("{} {}", SUIT_HANAFUDA[4], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[4], VALUES_HANAFUDA[3]), // x2
				// june
				format!("{} {}: Butterfly", SUIT_HANAFUDA[5], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[5], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[5], VALUES_HANAFUDA[3]), // x2
				// july
				format!("{} {}: Boar", SUIT_HANAFUDA[6], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[6], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[6], VALUES_HANAFUDA[3]), // x2
				// august
				format!("{} {}: Full moon", SUIT_HANAFUDA[7], VALUES_HANAFUDA[0]),
				format!("{} {}: Geese", SUIT_HANAFUDA[7], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[7], VALUES_HANAFUDA[3]), // x2
				// september
				format!("{} {}: Sake cup", SUIT_HANAFUDA[8], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[8], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[8], VALUES_HANAFUDA[3]), // x2
				// october
				format!("{} {}: Deer", SUIT_HANAFUDA[9], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[9], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[9], VALUES_HANAFUDA[3]), // x2
				// november
				format!(
					"{} {}: Ono no michikaze",
					SUIT_HANAFUDA[10], VALUES_HANAFUDA[0]
				),
				format!("{} {}: Swallow", SUIT_HANAFUDA[10], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[10], VALUES_HANAFUDA[2]),
				format!("{} {}: Lightning", SUIT_HANAFUDA[10], VALUES_HANAFUDA[3]), // x1
				// december
				format!(
					"{} {}: Chinese phoenix",
					SUIT_HANAFUDA[11], VALUES_HANAFUDA[0]
				),
				format!("{} {}", SUIT_HANAFUDA[11], VALUES_HANAFUDA[3]), // x3
				// snow
				format!(
					"{} {}: Ono no michikaze",
					SUIT_HANAFUDA[12], VALUES_HANAFUDA[0]
				),
				format!("{} {}: Swallow", SUIT_HANAFUDA[12], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[12], VALUES_HANAFUDA[2]),
				format!("{} {}: Lightning", SUIT_HANAFUDA[12], VALUES_HANAFUDA[3]), // x1
				// earth
				format!("{} {}", SUIT_HANAFUDA[13], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[13], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[13], VALUES_HANAFUDA[3]), // x2
				// heaven
				format!("{} {}", SUIT_HANAFUDA[14], VALUES_HANAFUDA[1]),
				format!("{} {}", SUIT_HANAFUDA[14], VALUES_HANAFUDA[2]),
				format!("{} {}", SUIT_HANAFUDA[14], VALUES_HANAFUDA[3]), // x2
			]],
			DeckVariant::Mahjong => vec![
				build_suits(
					&SUIT_MAHJONG,
					vec![&PIPS_ONE_NINE[..]].into_iter().flat_map(|s| s.iter()),
				), // x4
				MAHJONG_WINDS.iter().map(|s| format!("{s} wind")).collect(), // x4
				MAHJONG_DRAGONS
					.iter()
					.map(|s| format!("{s} dragon"))
					.collect(), // x4
				MAHJONG_SEASONS.iter().map(|s| s.to_string()).collect(),     // x1
				MAHJONG_FLOWERS.iter().map(|s| s.to_string()).collect(),     // x1
			],
		}
		.into_iter()
		.flat_map(|s| s.into_iter())
		.collect()
	}

	pub fn hand(self, n: usize) -> Vec<String> {
		self.set()
			.into_iter()
			.choose_multiple(&mut rand::thread_rng(), n)
	}
}

fn build_suit<'s, 'v>(suit: &'s str, values: &[String]) -> Vec<String>
where
	's: 'v,
{
	values
		.iter()
		.map(move |value| format!("{} of {}", value, suit))
		.collect()
}

fn build_suits<'s, 'v>(
	suits: &'s [&'s str],
	values: impl IntoIterator<Item = &'v &'v str>,
) -> Vec<String>
where
	's: 'v,
{
	let values: Vec<String> = values.into_iter().map(|v| v.to_string()).collect();
	suits
		.iter()
		.flat_map(|suit| build_suit(suit, &values))
		.collect()
}
