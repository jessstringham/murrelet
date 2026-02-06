use murrelet_common::MurreletColor;
use serde::{ser::SerializeSeq, Serializer};

use anyhow::Result;

pub fn serialize_color<S>(x: &MurreletColor, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hsva = x.into_hsva_components();

    let mut seq = serializer.serialize_seq(Some(hsva.len()))?;
    for number in &hsva {
        seq.serialize_element(number)?;
    }
    seq.end()
}