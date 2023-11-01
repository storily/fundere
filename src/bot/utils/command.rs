use twilight_model::application::interaction::application_command::{
	CommandDataOption, CommandOptionValue,
};

pub fn get_option<'o>(
	options: &'o [CommandDataOption],
	name: &str,
) -> Option<&'o CommandOptionValue> {
	options.iter().find_map(|opt| {
		if opt.name == name {
			Some(&opt.value)
		} else {
			None
		}
	})
}

pub fn get_string<'o>(options: &'o [CommandDataOption], name: &str) -> Option<&'o str> {
	get_option(options, name).and_then(|val| {
		if let CommandOptionValue::String(s) = val {
			Some(s.as_str())
		} else {
			None
		}
	})
}

pub fn get_integer(options: &[CommandDataOption], name: &str) -> Option<i64> {
	get_option(options, name).and_then(|val| {
		if let CommandOptionValue::Integer(i) = val {
			Some(*i)
		} else {
			None
		}
	})
}

pub fn get_boolean(options: &[CommandDataOption], name: &str) -> Option<bool> {
	get_option(options, name).and_then(|val| {
		if let CommandOptionValue::Boolean(b) = val {
			Some(*b)
		} else {
			None
		}
	})
}
