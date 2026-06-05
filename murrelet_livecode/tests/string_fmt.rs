use lerpable::Lerpable;
use murrelet_common::MurreletString;
use murrelet_livecode::livecode::{ControlMurreletString, LivecodeFromWorld};
use murrelet_livecode_derive::Livecode;

#[derive(Debug, Clone, Default, Livecode, Lerpable)]
pub struct HasString {
    name: MurreletString,
}

#[test]
fn control_string_raw_passthrough() {
    let c = ControlMurreletString::Raw("hello".to_string());
    let out = c.o_dummy().unwrap();
    assert_eq!(out.as_str(), "hello");
}

#[test]
fn control_string_fmt_constant_fill() {
    // Fmt { fmt: "s{}", fill: ["3"] } -> "s3"
    let yaml = r#"
fmt: "s{}"
fill: ["3"]
"#;
    let c: ControlMurreletString = serde_yaml::from_str(yaml).unwrap();
    let out = c.o_dummy().unwrap();
    assert_eq!(out.as_str(), "s3");
}

#[test]
fn control_string_fmt_two_fills() {
    let yaml = r#"
fmt: "{}_{}"
fill: ["1", "2"]
"#;
    let c: ControlMurreletString = serde_yaml::from_str(yaml).unwrap();
    let out = c.o_dummy().unwrap();
    assert_eq!(out.as_str(), "1_2");
}

#[test]
fn control_string_fmt_rounds_to_int() {
    let yaml = r#"
fmt: "s{}"
fill: ["3.9"]
"#;
    let c: ControlMurreletString = serde_yaml::from_str(yaml).unwrap();
    let out = c.o_dummy().unwrap();
    assert_eq!(out.as_str(), "s4");
}

#[test]
fn control_string_fmt_escaped_braces() {
    let yaml = r#"
fmt: "{{s{}}}"
fill: ["0"]
"#;
    let c: ControlMurreletString = serde_yaml::from_str(yaml).unwrap();
    let out = c.o_dummy().unwrap();
    assert_eq!(out.as_str(), "{s0}");
}

#[test]
fn bare_string_deserializes_as_raw() {
    // an untagged Raw should accept a plain scalar
    let c: ControlMurreletString = serde_yaml::from_str("just a literal").unwrap();
    let out = c.o_dummy().unwrap();
    assert_eq!(out.as_str(), "just a literal");
}

#[test]
fn derived_struct_with_string_field() {
    let yaml = r#"
name:
  fmt: "s{}"
  fill: ["3"]
"#;
    let c: ControlHasString = serde_yaml::from_str(yaml).unwrap();
    let out: HasString = c.o_dummy().unwrap();
    assert_eq!(out.name, MurreletString::new("s3"));

    let yaml_raw = r#"
name: literal_name
"#;
    let c2: ControlHasString = serde_yaml::from_str(yaml_raw).unwrap();
    let out2: HasString = c2.o_dummy().unwrap();
    assert_eq!(out2.name.as_str(), "literal_name");
}
