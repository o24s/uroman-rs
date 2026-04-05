use std::{fs::File, io::BufReader};
use uroman::{RomFormat, Uroman};

fn assert_uroman_output(input_path: &str, expected_output: &str) {
    let expected_output_normalized = expected_output.replace("\r\n", "\n");

    let uroman = Uroman::new();
    let mut buf = vec![];
    uroman
        .romanize_file(
            BufReader::new(File::open(input_path).unwrap()),
            &mut buf,
            None,
            RomFormat::Str,
            None,
            false,
            false,
        )
        .unwrap();

    let actual_output_normalized = String::from_utf8(buf).unwrap().replace("\r\n", "\n");

    assert_eq!(actual_output_normalized, expected_output_normalized);
}

#[test]
fn test_multi_script_romanization() {
    assert_uroman_output(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test/multi-script.txt"),
        include_str!("test/multi-script.uroman-ref.txt"),
    );
}
