use std::{collections::HashSet, str::FromStr};

use miette::{miette, Report, Result};
use rand::seq::IteratorRandom;

pub const SUIT_ENGLISH: [&str; 4] = ["Clubs", "Diamonds", "Hearts", "Spades"];
pub const SUIT_FRENCH: [&str; 4] = ["Clovers", "Tiles", "Hearts", "Pikes"];
pub const SUIT_GERMAN: [&str; 4] = ["Hearts", "Bells", "Acorns", "Leaves"];
pub const SUIT_ITALIAN: [&str; 4] = ["Cups", "Coins", "Clubs", "Swords"];
pub const SUIT_SPANISH: [&str; 4] = SUIT_ITALIAN;
pub const SUIT_SWISS: [&str; 4] = ["Roses", "Bells", "Acorns", "Shields"];
pub const SUIT_TAROT: [&str; 5] = ["Cups", "Coins", "Batons", "Swords", "Arcana"];
pub const SUIT_NOUVEAU: [&str; 5] = ["Clovers", "Tiles", "Hearts", "Pikes", "Honours"];
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
pub const FACES_MINCHIATE: [&str; 4] = FACES_NOUVEAU;
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
	FOOL,
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
		.flat_map(|s| s.into_iter())
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
	pub fn random(self) -> String {
		let set = match self {
			ValueVariant::All => todo!(),
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
		.flat_map(|s| s.into_iter())
		.collect::<HashSet<&&str>>();

		set.into_iter()
			.choose(&mut rand::thread_rng())
			.unwrap()
			.to_string()
	}
}

