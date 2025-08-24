#![allow(dead_code)]

use crate::{DataType as BaseDataType, DataValue as BaseDataValue, parser::reader::Reader};

pub struct Definition {
    pub message_type: u8,
    pub fields: Vec<DefinitionField>,
}

pub struct DefinitionField {
    pub definition_number: u8,
    pub parse: fn(&mut Reader<std::vec::IntoIter<u8>>) -> DataValue,
}

#[derive(Debug, PartialEq)]
pub enum DataValue {
    Base(BaseDataValue),
    Enum(Enums),
}

#[derive(Debug, PartialEq)]
pub enum Enums {
    Activity(Activity),
}

#[derive(Debug, PartialEq)]
pub enum Activity {
    Running,
    Cycling,
}

impl Activity {
    pub fn from(byte: u8) -> Activity {
        Activity::Running
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() {
        let mut iter = Reader::new(3, 0, vec![1, 2, 3].into_iter());

        fn parse(iter: &mut Reader<std::vec::IntoIter<u8>>) -> DataValue {
            DataValue::Base(BaseDataValue::Uint8(iter.next_u8().unwrap()))
        }

        let definition = DefinitionField {
            definition_number: 0,
            parse,
        };

        let value = (definition.parse)(&mut iter);

        assert_eq!(value, DataValue::Base(BaseDataValue::Uint8(1)));
    }
}
