//! Simple settings exposed by a manifest (`settings:`), written into the tool's own
//! YAML/JSON config file as dotted keys. Comments inside YAML files are not preserved
//! (the original is always backed up first).

use crate::engine::{Plan, Step};
use crate::manifest::{Manifest, Setting};
use crate::os::Os;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

fn parse(text: &str, format: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    match format {
        "json" => serde_json::from_str(text).map_err(|e| format!("invalid JSON config: {e}")),
        "yaml" => serde_yaml::from_str(text).map_err(|e| format!("invalid YAML config: {e}")),
        other => Err(format!("unsupported_format:{other}")),
    }
}

pub fn config_get(text: &str, format: &str, key: &str) -> Option<Value> {
    let mut cur = parse(text, format).ok()?;
    for part in key.split('.') {
        cur = cur.get(part)?.clone();
    }
    Some(cur)
}

pub fn config_set(text: &str, format: &str, key: &str, value: &Value) -> Result<String, String> {
    let mut root = parse(text, format)?;
    if !root.is_object() {
        return Err("config root is not a mapping".into());
    }
    let parts: Vec<&str> = key.split('.').collect();
    let mut cur = &mut root;
    for (i, part) in parts.iter().enumerate() {
        let obj = cur.as_object_mut().ok_or_else(|| format!("{key}: parent is not a mapping"))?;
        if i == parts.len() - 1 {
            obj.insert((*part).into(), value.clone());
            break;
        }
        cur = obj.entry(*part).or_insert_with(|| Value::Object(Map::new()));
    }
    match format {
        "json" => serde_json::to_string_pretty(&root).map(|s| s + "\n").map_err(|e| e.to_string()),
        _ => serde_yaml::to_string(&root).map_err(|e| e.to_string()),
    }
}

fn format_of(m: &Manifest) -> Option<String> {
    m.config.as_ref().and_then(|c| c.format.clone())
}

fn config_file(os: &dyn Os, m: &Manifest) -> Option<String> {
    let paths = &m.config.as_ref()?.paths;
    paths.iter().find(|p| os.exists(&os.expand(p))).or(paths.first()).cloned()
}

/// Value as shown in the UI: suffix stripped from numbers, default when unset.
fn from_file(s: &Setting, v: Value) -> Value {
    match (&s.suffix, &v) {
        (Some(sfx), Value::String(t)) => t.strip_suffix(sfx.as_str()).and_then(|n| n.trim().parse::<f64>().ok()).map(|n| json!(n)).unwrap_or(v),
        _ => v,
    }
}

pub fn read_all(os: &dyn Os, m: &Manifest) -> BTreeMap<String, Value> {
    let text = config_file(os, m).and_then(|f| os.read(&os.expand(&f)).ok().flatten()).unwrap_or_default();
    let fmt = format_of(m).unwrap_or_default();
    m.settings
        .iter()
        .map(|s| {
            let v = config_get(&text, &fmt, &s.key).map(|v| from_file(s, v)).unwrap_or_else(|| s.default.clone());
            (s.key.clone(), v)
        })
        .collect()
}

pub fn plan_set(os: &dyn Os, m: &Manifest, key: &str, value: Value) -> Result<Plan, String> {
    let s = m.settings.iter().find(|s| s.key == key).ok_or("unknown_setting")?;
    let stored = match (s.kind.as_str(), &value) {
        ("number", Value::Number(n)) => {
            let x = n.as_f64().ok_or("invalid_value")?;
            if s.min.is_some_and(|min| x < min) || s.max.is_some_and(|max| x > max) {
                return Err("invalid_value".into());
            }
            match &s.suffix {
                Some(sfx) => Value::String(format!("{n}{sfx}")),
                None => value.clone(),
            }
        }
        ("boolean", Value::Bool(_)) | ("string" | "color", Value::String(_)) => value.clone(),
        _ => return Err("invalid_value".into()),
    };
    let file = config_file(os, m).ok_or("no_config")?;
    let format = format_of(m).ok_or("no_config")?;
    Ok(Plan {
        tool: m.id.clone(),
        action: "settings".into(),
        steps: vec![Step::SetConfigValue { tool: m.id.clone(), file, format, key: key.into(), value: stored }],
    })
}
