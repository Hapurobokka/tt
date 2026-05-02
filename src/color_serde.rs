use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = match color {
        Color::White => "white",
        Color::Black => "black",
        Color::Red => "red",
        Color::Blue => "blue",
        Color::Green => "green",
        Color::Yellow => "yellow",
        Color::Magenta => "magenta",
        Color::Gray => "gray",
        _ => &color.to_string(),
    };
    s.serialize(serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "white" => Ok(Color::White),
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "blue" => Ok(Color::Blue),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "magenta" => Ok(Color::Magenta),
        "gray" => Ok(Color::Gray),
        other => Err(serde::de::Error::custom(format!(
            "Color desconocido: {other}"
        ))),
    }
}
