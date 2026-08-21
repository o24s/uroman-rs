extern crate uroman;

use uroman::{RomFormat, Uroman, rom_format};

#[rustfmt::skip]
fn main() {
    let uroman = Uroman::new();

    let s = "こんにちは、ユーロマン！";
    let lcode = None;

    // Str output
    let result = uroman
        .romanize_string::<rom_format::Str>(s, lcode)
        .to_string();

    let result_f = uroman.romanize_with_format(
        s,
        lcode,
        None, // `None` defaults to `RomFormat::Str`.
        // RomFormat::Str,
    ).to_string();

    // This unwrap is safe because `RomFormat::Str` never fails.
    // If you prefer to avoid `.unwrap()`, use `romanize_string`.
    assert_eq!(result, result_f.unwrap());

    println!("{result}");

    // Lattice output
    let result = uroman
        .romanize_string::<rom_format::Lattice>(s, lcode)
        .to_string()
        .unwrap();

    let result_f = uroman
        .romanize_with_format(s, lcode, Some(RomFormat::Lattice))
        .to_string()
        .unwrap();

    assert_eq!(result, result_f);

    println!("{result}");
}
