//! Compiles the uroman data files under `data/` into a single zero-copy
//! `rkyv` archive (`$OUT_DIR/dicts.rkyv`) that the library embeds and reads
//! without any startup parsing.
//!
//! The dictionary model and parsers are shared with the library itself via
//! `#[path]` modules, so build-time and runtime always agree on the layout.

use std::env;
use std::path::Path;

#[path = "src/dicts/strutil.rs"]
mod strutil;

#[path = "src/dicts/model.rs"]
mod model;

#[path = "src/dicts/parse.rs"]
mod parse;

fn main() {
    println!("cargo:rerun-if-changed=data");
    println!("cargo:rerun-if-changed=src/dicts/strutil.rs");
    println!("cargo:rerun-if-changed=src/dicts/model.rs");
    println!("cargo:rerun-if-changed=src/dicts/parse.rs");

    let dicts = parse::build(&parse::DataFiles {
        romanization_auto_table: include_str!("data/romanization-auto-table.txt"),
        unicode_data_overwrite: include_str!("data/UnicodeDataOverwrite.txt"),
        romanization_table: include_str!("data/romanization-table.txt"),
        chinese_to_pinyin: include_str!("data/Chinese_to_Pinyin.txt"),
        scripts: include_str!("data/Scripts.txt"),
        unicode_data_props: [
            include_str!("data/UnicodeDataProps.txt"),
            include_str!("data/UnicodeDataPropsCJK.txt"),
            include_str!("data/UnicodeDataPropsHangul.txt"),
        ],
        num_props: include_str!("data/NumProps.jsonl"),
        unicode_data: include_str!("data/UnicodeData.txt"),
    });

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&dicts)
        .expect("failed to serialize the dictionary archive");

    let out_path = Path::new(&env::var("OUT_DIR").unwrap()).join("dicts.rkyv");
    std::fs::write(&out_path, &bytes).expect("failed to write the dictionary archive");
}
