use std::collections::{hash_map::IntoIter, HashMap};

use serenity::all::{CommandDataOption, CommandDataOptionValue};

#[derive(Debug, Default)]
pub struct Subcommand<'a> {
    pub name: &'a str,
    pub options: SubcommandOptions<'a>,
}

#[derive(Debug, Default)]
pub struct SubcommandOptions<'a> {
    pub inner: HashMap<&'a str, &'a CommandDataOptionValue>,
}

impl<'a> Subcommand<'a> {
    pub fn from_command_data_option(command_data_option: &'a CommandDataOption) -> Option<Self> {
        match &command_data_option.value {
            CommandDataOptionValue::SubCommand(subcommand) => Some(Self {
                name: &command_data_option.name,
                options: SubcommandOptions::new(subcommand)
            }),
            _ => None
        }
    }
}

impl<'a> IntoIterator for SubcommandOptions<'a> {
    type Item = (&'a str, &'a CommandDataOptionValue);

    type IntoIter = IntoIter<&'a str, &'a CommandDataOptionValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> SubcommandOptions<'a> {
    pub fn new(options: &'a [CommandDataOption]) -> Self {
        let inner = options
            .iter()
            .map(|v| (v.name.as_str(), &v.value))
            .collect();

        Self { inner }
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<&&'a CommandDataOptionValue> {
        self.inner.get(key.as_ref())
    }
}
