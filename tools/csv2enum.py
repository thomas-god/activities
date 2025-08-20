#!/usr/bin/env python3
import csv
import sys


def snake_to_camel(string: str) -> str:
    words = string.strip().lower().split("_")
    return "".join(word.capitalize() for word in words if word)


if __name__ == "__main__":
    with open(sys.argv[1]) as file:
        reader = csv.reader(file, delimiter="\t")
        rows = [row for row in reader if row[0] != ""]

        variants = [snake_to_camel(row[1]) for row in rows]
        mappings = [f"{row[0]} => Self::{snake_to_camel(row[1])}" for row in rows]

        enum_name = sys.argv[2]
        enum_code = f"""
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum {enum_name} {{
{",\n".join(variants)},
    Unknown(u8),
}}

impl From<u8> for {enum_name} {{
    fn from(value: u8) -> Self {{
        match value {{
{",\n".join(mappings)},
            _ => Self::Unknown(value)
        }}
    }}
}}"""

        print(enum_code)
