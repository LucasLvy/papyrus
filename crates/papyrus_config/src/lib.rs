use std::collections::{BTreeMap, HashMap};
use std::ops::IndexMut;
use std::time::Duration;

use clap::parser::MatchesError;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

pub type ParamPath = String;
pub type Description = String;

pub mod command;
#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;

pub const DEFAULT_CHAIN_ID: &str = "SN_MAIN";

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SerializedParam {
    pub description: String,
    pub value: Value,
}

#[derive(thiserror::Error, Debug)]
pub enum SubConfigError {
    #[error(transparent)]
    CommandInput(#[from] clap::error::Error),
    #[error(transparent)]
    MissingParam(#[from] serde_json::Error),
    #[error(transparent)]
    CommandMatches(#[from] MatchesError),
    #[error("Insert a new param is not allowed.")]
    ParamNotFound { param_path: String },
}
/// Serialization for configs.
pub trait SerializeConfig {
    /// Conversion of a configuration to a mapping of flattened parameters to their descriptions and
    /// values.
    /// Note, in the case of a None sub configs, its elements will not included in the flatten map.
    fn dump(&self) -> BTreeMap<ParamPath, SerializedParam>;
}

/// Deserializes config from flatten JSON.
/// For an explanation of `for<'a> Deserialize<'a>` see
/// `<https://doc.rust-lang.org/nomicon/hrtb.html>`.
pub fn load<T: for<'a> Deserialize<'a>>(
    config_dump: &BTreeMap<ParamPath, SerializedParam>,
) -> Result<T, SubConfigError> {
    let mut nested_map = json!({});
    for (param_path, serialized_param) in config_dump {
        let mut entry = &mut nested_map;
        for config_name in param_path.split('.') {
            entry = entry.index_mut(config_name);
        }
        *entry = serialized_param.value.clone();
    }
    Ok(serde_json::from_value(nested_map)?)
}

pub fn update_config_map(
    config_map: &mut BTreeMap<ParamPath, SerializedParam>,
    param_path: &str,
    new_value: Value,
) -> Result<(), SubConfigError> {
    let Some(serialized_param) = config_map.get_mut(param_path) else {
        return Err(SubConfigError::ParamNotFound{param_path: param_path.to_string()});
    };
    serialized_param.value = new_value;
    Ok(())
}

/// Appends `sub_config_name` to the ParamPath for each entry in `sub_config_dump`.
/// In order to load from a dump properly, `sub_config_name` must match the field's name for the
/// struct this function is called from.
pub fn append_sub_config_name(
    sub_config_dump: BTreeMap<ParamPath, SerializedParam>,
    sub_config_name: &str,
) -> BTreeMap<ParamPath, SerializedParam> {
    BTreeMap::from_iter(
        sub_config_dump
            .into_iter()
            .map(|(field_name, val)| (format!("{}.{}", sub_config_name, field_name), val)),
    )
}

/// Serializes a single param of a config.
/// The returned pair is designed to be an input to a dumped config map.
pub fn ser_param<T: Serialize>(
    name: &str,
    value: &T,
    description: &str,
) -> (String, SerializedParam) {
    (name.to_owned(), SerializedParam { description: description.to_owned(), value: json!(value) })
}

pub fn deserialize_milliseconds_to_duration<'de, D>(de: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs: u64 = Deserialize::deserialize(de)?;
    Ok(Duration::from_millis(secs))
}

/// Serializes a map to "k1:v1 k2:v2" string structure.
pub fn serialize_optional_map(optional_map: &Option<HashMap<String, String>>) -> String {
    match optional_map {
        None => "".to_owned(),
        Some(map) => map.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<String>>().join(" "),
    }
}

/// Deserializes a map from "k1:v1 k2:v2" string structure.
pub fn deserialize_optional_map<'de, D>(de: D) -> Result<Option<HashMap<String, String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_str: String = Deserialize::deserialize(de)?;
    if raw_str.is_empty() {
        return Ok(None);
    }

    let mut map = HashMap::new();
    for raw_pair in raw_str.split(' ') {
        let split: Vec<&str> = raw_pair.split(':').collect();
        if split.len() != 2 {
            return Err(D::Error::custom(format!(
                "pair \"{}\" is not valid. The Expected format is name:value",
                raw_pair
            )));
        }
        map.insert(split[0].to_string(), split[1].to_string());
    }
    Ok(Some(map))
}
