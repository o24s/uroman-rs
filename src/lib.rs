//! Main library for the uroman-rs project.
//!
//! This library provides the `Uroman` struct, which is the main entry point
//! for romanizing strings. It loads romanization rules from data files and
//! applies them to input text.

#![allow(clippy::too_many_arguments)]

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;
use std::any::TypeId;
use std::fmt;
use std::io::{self, BufRead, Write};
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock};
use thiserror::Error;

use crate::core::UromanInner;
pub use crate::edge::Edge;
use crate::lattice::Lattice;
use crate::utils::decode_unicode_escapes;

mod core;
mod decompositions;
mod edge;
mod lattice;
mod rom_rule;
mod utils;

use rom_rule::RomRule;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RomFormat {
    #[default]
    Str,
    Edges,
    Alts,
    Lattice,
}

pub(crate) use rom_format::IsEdgeFormat;
pub(crate) use rom_format::IsStrFormat;
pub(crate) use rom_format::RomFormatType;

pub mod rom_format {
    use crate::RomanizationError;

    pub struct Str;
    pub struct Edges;
    pub struct Alts;
    pub struct Lattice;

    pub trait RomFormatType {
        type Output;
    }

    impl RomFormatType for Str {
        type Output = String;
    }

    impl RomFormatType for Edges {
        type Output = Result<String, RomanizationError>;
    }

    impl RomFormatType for Alts {
        type Output = Result<String, RomanizationError>;
    }

    impl RomFormatType for Lattice {
        type Output = Result<String, RomanizationError>;
    }

    pub trait IsStrFormat {}
    pub trait IsEdgeFormat {}

    impl IsStrFormat for Str {}
    impl IsEdgeFormat for Edges {}
    impl IsEdgeFormat for Alts {}
    impl IsEdgeFormat for Lattice {}
}

pub struct RomanizationOutput<F: RomFormatType> {
    pub(crate) result: RomanizationResult,
    _marker: PhantomData<F>,
}

impl<F: RomFormatType> RomanizationOutput<F> {
    pub fn to_string(self) -> F::Output
    where
        F::Output: From<RomanizationResult>,
    {
        self.result.into()
    }
}

impl<F: RomFormatType + IsStrFormat> RomanizationOutput<F> {
    pub fn as_str(&self) -> &str {
        match &self.result {
            RomanizationResult::Str(s) => s,
            RomanizationResult::Edges(_) => unreachable!(),
        }
    }
}

impl<F: RomFormatType + IsEdgeFormat> RomanizationOutput<F> {
    pub fn to_edges(self) -> Vec<Edge> {
        match self.result {
            RomanizationResult::Edges(edges) => edges,
            RomanizationResult::Str(_) => {
                panic!("Logic error: An edge-based format produced a string result.")
            }
        }
    }
}

impl<F: RomFormatType + IsEdgeFormat> RomanizationOutput<F> {
    pub fn as_edges(&self) -> &[Edge] {
        match &self.result {
            RomanizationResult::Edges(edges) => edges,
            RomanizationResult::Str(_) => {
                panic!("Logic error: An edge-based format produced a string result.")
            }
        }
    }
}

impl<'a, F> IntoIterator for &'a RomanizationOutput<F>
where
    F: RomFormatType + IsEdgeFormat,
{
    type Item = &'a Edge;
    type IntoIter = std::slice::Iter<'a, Edge>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_edges().iter()
    }
}

impl<F> IntoIterator for RomanizationOutput<F>
where
    F: RomFormatType + IsEdgeFormat,
{
    type Item = Edge;
    type IntoIter = std::vec::IntoIter<Edge>;

    fn into_iter(self) -> Self::IntoIter {
        self.to_edges().into_iter()
    }
}

impl From<RomanizationResult> for String {
    fn from(res: RomanizationResult) -> Self {
        match res {
            RomanizationResult::Str(s) => s,
            _ => panic!("Expected RomanizationResult::Str, but got Edges"),
        }
    }
}

impl From<RomanizationResult> for Result<String, RomanizationError> {
    fn from(res: RomanizationResult) -> Self {
        match res {
            RomanizationResult::Edges(edges) => Ok(serde_json::to_string_pretty(&edges)?),
            _ => Err(RomanizationError::InternalError(
                "Mismatched result".to_string(),
            )),
        }
    }
}

impl<F: RomFormatType> fmt::Display for RomanizationOutput<F>
where
    F::Output: From<RomanizationResult>,
    F::Output: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.result.to_string() {
            Ok(s) => write!(f, "{}", s),
            Err(e) => {
                write!(f, "Error: {:?}", e)
            }
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd)]
#[serde(untagged)]
pub enum RomanizationResult {
    Str(String),
    Edges(Vec<Edge>),
}

impl RomanizationResult {
    pub fn to_string(&self) -> Result<String, RomanizationError> {
        match self {
            RomanizationResult::Str(s) => Ok(s.clone()),
            RomanizationResult::Edges(edges) => Ok(serde_json::to_string_pretty(edges)?),
        }
    }
}

impl fmt::Display for RomanizationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_string() {
            Ok(s) => write!(f, "{}", s),
            Err(e) => write!(f, "Error: {:?}", e),
        }
    }
}

#[derive(Error, Debug)]
pub enum RomanizationError {
    #[error("Failed to serialize the result to JSON: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("Internal logic error: {0}")]
    InternalError(String),
}

static GLOBAL_UROMAN_INNER: LazyLock<Arc<UromanInner>> =
    LazyLock::new(|| Arc::new(UromanInner::new()));

/// The main struct for romanization.
///
/// It holds the romanization rules and provides methods to romanize strings.
/// This corresponds to the `Uroman` class in the Python implementation.
#[derive(Debug, Clone, Default)]
pub struct Uroman {
    inner: Arc<UromanInner>,
}

impl Uroman {
    pub fn new() -> Self {
        Self {
            inner: Arc::clone(&GLOBAL_UROMAN_INNER),
        }
    }

    /// Romanizes a given string.
    ///
    /// # Arguments
    ///
    /// lcode: [ISO 639-3 language code](https://www.loc.gov/standards/iso639-2/php/code_list.php)
    /// (e.g., eng, jpn, hin, ara, zho)
    ///
    /// # Example
    /// ```
    /// # use uroman::{Uroman, rom_format};
    /// # let uroman = Uroman::new();
    /// let lcode = None;
    /// let result = uroman.romanize_string::<rom_format::Str>("ᚺᚨᛚᛚᛟ ᚹᛟᚱᛚᛞ", lcode);
    ///
    /// let str = result.to_string();
    ///
    /// println!("{str}");
    /// ```
    pub fn romanize_string<F: RomFormatType + 'static>(
        &self,
        s: &str,
        lcode: Option<&str>,
    ) -> RomanizationOutput<F> {
        let mut lat = Lattice::new(s, &self.inner, lcode);

        lat.pick_tibetan_vowel_edge();
        lat.prep_braille();
        lat.add_romanization();
        lat.add_numbers();
        lat.add_braille_numbers();
        lat.add_rom_fall_back_singles();

        let type_id = TypeId::of::<F>();

        let result = if type_id == TypeId::of::<rom_format::Str>() {
            let best_edges = lat.best_rom_edge_path(0, s.chars().count(), false);
            RomanizationResult::Str(best_edges.iter().map(|edge| edge.txt()).collect::<String>())
        } else if type_id == TypeId::of::<rom_format::Edges>() {
            RomanizationResult::Edges(lat.best_rom_edge_path(0, s.chars().count(), false))
        } else if type_id == TypeId::of::<rom_format::Alts>() {
            let mut best_edges = lat.best_rom_edge_path(0, s.chars().count(), false);
            lat.add_alternatives(&mut best_edges);

            RomanizationResult::Edges(best_edges)
        } else if type_id == TypeId::of::<rom_format::Lattice>() {
            let mut all_edges = lat.all_edges(0, s.chars().count());
            lat.add_alternatives(&mut all_edges);

            RomanizationResult::Edges(all_edges)
        } else {
            unreachable!("Unknown RomFormatType provided");
        };

        RomanizationOutput {
            result,
            _marker: PhantomData,
        }
    }

    /// Decodes Unicode escape sequences before performing romanization.
    ///
    /// # Arguments
    ///
    /// lcode: [ISO 639-3 language code](https://www.loc.gov/standards/iso639-2/php/code_list.php)
    /// (e.g., eng, jpn, hin, ara, zho)
    ///
    /// # Example
    /// ```
    /// # use uroman::{Uroman, rom_format};
    /// # let uroman = Uroman::new();
    /// let lcode = None;
    /// let result = uroman.romanize_escaped::<rom_format::Str>("ᚺᚨᛚᛚᛟ ᚹᛟᚱᛚᛞ", lcode);
    ///
    /// let str = result.to_string();
    ///
    /// println!("{str}");
    /// ```
    pub fn romanize_escaped<F: RomFormatType + 'static>(
        &self,
        s: &str,
        lcode: Option<&str>,
    ) -> RomanizationOutput<F> {
        let s = decode_unicode_escapes(s);
        self.romanize_string::<F>(s.as_str(), lcode)
    }

    /// Decodes Unicode escape sequences and then romanizes the string using the specified `RomFormat`.
    ///
    /// # Arguments
    ///
    /// lcode: [ISO 639-3 language code](https://www.loc.gov/standards/iso639-2/php/code_list.php)
    /// (e.g., eng, jpn, hin, ara, zho)
    ///
    /// # Example
    /// ```
    /// # use uroman::{Uroman, RomFormat};
    /// # let uroman = Uroman::new();
    /// let lcode = None;
    /// let result = uroman.romanize_with_format(
    ///     "ᚺᚨᛚᛚᛟ ᚹᛟᚱᛚᛞ",
    ///     lcode,
    ///     Some(RomFormat::Edges),
    /// );
    ///
    /// let str = result.to_string().unwrap();
    ///
    /// println!("{str}");
    /// ```
    pub fn romanize_escaped_with_format(
        &self,
        s: &str,
        lcode: Option<&str>,
        rom_format: Option<RomFormat>,
    ) -> RomanizationResult {
        let s = decode_unicode_escapes(s);
        self.romanize_with_format(&s, lcode, rom_format)
    }

    /// Romanizes a given string using `RomFormat`.
    ///
    /// # Arguments
    ///
    /// lcode: [ISO 639-3 language code](https://www.loc.gov/standards/iso639-2/php/code_list.php)
    /// (e.g., eng, jpn, hin, ara, zho)
    ///
    /// # Example
    /// ```
    /// # use uroman::Uroman;
    /// # let uroman = Uroman::new();
    /// let lcode = None;
    /// let result = uroman.romanize_with_format(
    ///     "ᚺᚨᛚᛚᛟ ᚹᛟᚱᛚᛞ",
    ///     lcode,
    ///     None, // `None` defaults to `RomFormat::Str`.
    ///     // RomFormat::Str,
    /// );
    ///
    /// let str = result.to_string().unwrap();
    ///
    /// println!("{str}");
    /// ```
    pub fn romanize_with_format(
        &self,
        s: &str,
        lcode: Option<&str>,
        rom_format: Option<RomFormat>,
    ) -> RomanizationResult {
        let rom_format = rom_format.unwrap_or(RomFormat::Str);

        match rom_format {
            RomFormat::Str => {
                let str = self.romanize_string::<rom_format::Str>(s, lcode);
                RomanizationResult::Str(str.to_string())
            }
            RomFormat::Edges => self.romanize_string::<rom_format::Edges>(s, lcode).result,
            RomFormat::Alts => self.romanize_string::<rom_format::Alts>(s, lcode).result,
            RomFormat::Lattice => self.romanize_string::<rom_format::Lattice>(s, lcode).result,
        }
    }

    /// Romanizes a stream of text line by line and writes the output to another stream.
    ///
    /// This method efficiently processes large amounts of text by reading from a buffered
    /// reader and writing to a writer without loading the entire content into memory.
    ///
    /// # Arguments
    ///
    /// lcode: [ISO 639-3 language code](https://www.loc.gov/standards/iso639-2/php/code_list.php)
    /// (e.g., eng, jpn, hin, ara, zho)
    ///
    /// # Errors
    ///
    /// This function will return an `io::Error` if any I/O operation fails during
    /// reading from the `reader` or writing to the `writer`.
    pub fn romanize_file<R: BufRead, W: Write>(
        &self,
        mut reader: R,
        mut writer: W,
        lcode: Option<&str>,
        rom_format: RomFormat,
        max_lines: Option<usize>,
        decode_unicode: bool,
        silent: bool,
    ) -> Result<(), RomanizationError> {
        let mut line_number = 0;
        let mut non_utf8_chars_total = 0;
        let mut n_error_messages_output = 0;
        let max_n_error_messages = 10;

        let mut buffer = vec![];
        let default_lcode = lcode;
        let lcode_directive = "::lcode ";

        while reader.read_until(b'\n', &mut buffer)? > 0 {
            line_number += 1;

            let original_len = buffer.len();
            let line_str = String::from_utf8_lossy(&buffer);
            let replaced_len = line_str.len();
            if replaced_len < original_len {
                non_utf8_chars_total += 1;
                if n_error_messages_output < max_n_error_messages {
                    eprintln!(
                        "Detected encoding error on line {line_number}: non-UTF-8 characters were replaced."
                    );
                    n_error_messages_output += 1;
                } else if n_error_messages_output == max_n_error_messages {
                    eprintln!("Too many encoding errors. No further errors reported.");
                    n_error_messages_output += 1;
                }
            }
            let mut line_trimmed = &*line_str;

            if line_trimmed.ends_with('\n') {
                line_trimmed = &line_trimmed[..line_trimmed.len() - 1];
            }
            if line_trimmed.ends_with('\r') {
                line_trimmed = &line_trimmed[..line_trimmed.len() - 1];
            }

            if let Some(rest_of_line) = line_trimmed.strip_prefix(lcode_directive) {
                let parts: Vec<&str> = rest_of_line.splitn(2, char::is_whitespace).collect();
                let (lcode, text_to_romanize) =
                    (parts.first().cloned(), parts.get(1).cloned().unwrap_or(""));

                let result = if decode_unicode {
                    self.romanize_escaped_with_format(text_to_romanize, lcode, Some(rom_format))
                } else {
                    self.romanize_with_format(text_to_romanize, lcode, Some(rom_format))
                };

                match rom_format {
                    RomFormat::Str => {
                        let prefix = format!("{}{}{} ", lcode_directive, lcode.unwrap_or(""), "");
                        let output = prefix + &result.to_string().unwrap();
                        writeln!(writer, "{output}")?;
                    }
                    _ => {
                        let meta_edge = format!(r#"[0,0,"","lcode: {}"]"#, lcode.unwrap_or(""));
                        let result_json = result.to_string().unwrap();
                        if let Some(stripped) = result_json.strip_prefix('[') {
                            writeln!(writer, "[{meta_edge},{stripped}")?;
                        } else {
                            writeln!(writer, "{result_json}")?;
                        }
                    }
                }
            } else {
                let result = if decode_unicode {
                    self.romanize_escaped_with_format(line_trimmed, default_lcode, Some(rom_format))
                } else {
                    self.romanize_with_format(line_trimmed, default_lcode, Some(rom_format))
                };
                let output = result.to_string().expect("JSON serialization failed");
                writeln!(writer, "{output}")?;
            }

            if let Some(max) = max_lines
                && line_number >= max
            {
                break;
            }
            buffer.clear();
        }

        if !silent && line_number > 0 {
            eprintln!();
        }
        if non_utf8_chars_total > 0 {
            eprintln!("Total number of lines with non-UTF-8 characters: {non_utf8_chars_total}");
        }

        writer.flush()?;
        Ok(())
    }

    /// Romanizes a stream of text line by line in parallel for maximum performance.
    ///
    /// This version reads the entire input into memory to process lines concurrently using
    /// multiple CPU cores. It is significantly faster than `romanize_file` but requires
    /// more memory. For very large files, consider using the sequential `romanize_file`.
    ///
    /// The output order is preserved.
    ///
    /// # Differences from `romanize_file`
    ///
    /// * Memory Usage: Loads the entire file into memory. May fail on files larger than RAM.
    /// * Error Reporting: Does not warn about invalid UTF-8 characters.
    ///
    /// # Arguments
    ///
    /// lcode: [ISO 639-3 language code](https://www.loc.gov/standards/iso-639-2/php/code_list.php)
    /// (e.g., eng, jpn, hin, ara, zho)
    ///
    /// # Errors
    ///
    /// This function will return an `io::Error` if any I/O operation fails during
    /// reading from the `reader` or writing to the `writer`.
    pub fn romanize_file_parallel<R: BufRead, W: Write>(
        &self,
        reader: R,
        mut writer: W,
        lcode: Option<&str>,
        rom_format: RomFormat,
        max_lines: Option<usize>,
        decode_unicode: bool,
        silent: bool,
    ) -> Result<(), RomanizationError> {
        let mut lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

        if let Some(max) = max_lines {
            lines.truncate(max);
        }

        let line_count = lines.len();
        let default_lcode = lcode;
        let lcode_directive = "::lcode ";
        // UTF-8 error handling is simplified as `lines()` replaces invalid sequences.
        // The original byte-level diff check is not replicated here.

        let results: Vec<String> = lines
            .par_iter()
            .map(|line| {
                if let Some(rest_of_line) = line.strip_prefix(lcode_directive) {
                    let parts: Vec<&str> = rest_of_line.splitn(2, char::is_whitespace).collect();
                    let (lcode, text_to_romanize) =
                        (parts.first().cloned(), parts.get(1).cloned().unwrap_or(""));

                    let result = if decode_unicode {
                        self.romanize_escaped_with_format(text_to_romanize, lcode, Some(rom_format))
                    } else {
                        self.romanize_with_format(text_to_romanize, lcode, Some(rom_format))
                    };

                    let output = result.to_string().unwrap_or_default();

                    match rom_format {
                        RomFormat::Str => {
                            format!(
                                "{}{}{} {}",
                                lcode_directive,
                                lcode.unwrap_or(""),
                                "",
                                output
                            )
                        }
                        _ => {
                            let meta_edge = format!(r#"[0,0,"","lcode: {}"]"#, lcode.unwrap_or(""));
                            if let Some(stripped) = output.strip_prefix('[') {
                                format!("[{meta_edge},{stripped}")
                            } else {
                                output
                            }
                        }
                    }
                } else {
                    let result = if decode_unicode {
                        self.romanize_escaped_with_format(line, default_lcode, Some(rom_format))
                    } else {
                        self.romanize_with_format(line, default_lcode, Some(rom_format))
                    };
                    result.to_string().unwrap_or_default()
                }
            })
            .collect();

        for output in results {
            writeln!(writer, "{}", output)?;
        }

        if !silent && line_count > 0 {
            eprintln!();
        }

        writer.flush()?;
        Ok(())
    }
}
