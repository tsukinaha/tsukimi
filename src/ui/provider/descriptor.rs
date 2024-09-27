use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Descriptor {
    pub content: String,
    pub type_: DescriptorType,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum DescriptorType {
    String,
    Regex,
}

impl DescriptorType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "String" => Self::String,
            "Regex" => Self::Regex,
            _ => panic!("Invalid DescriptorType"),
        }
    }

    pub fn from_u32(u: u32) -> Self {
        match u {
            0 => Self::String,
            1 => Self::Regex,
            _ => panic!("Invalid DescriptorType"),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::String => "String".to_string(),
            Self::Regex => "Regex".to_string(),
        }
    }
}

pub trait VecSerialize<T> {
    fn to_string(&self) -> String;
}

impl VecSerialize<Descriptor> for Vec<Descriptor> {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).expect("Failed to serialize Vec<Descriptor>")
    }
}

impl Descriptor {
    pub fn new(content: String, type_: DescriptorType) -> Self {
        Self { content, type_ }
    }
}
