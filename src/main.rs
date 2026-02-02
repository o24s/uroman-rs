//! Command-line interface for uroman-rs.

use clap::{Parser, ValueEnum};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::io::{self, BufRead, BufReader, BufWriter, IsTerminal, Write};
use std::path::PathBuf;
use std::{fs, time};
use thiserror::Error;
use unicode_width::UnicodeWidthStr;
use uroman::{RomFormat, RomanizationError, Uroman, rom_format};

#[derive(ValueEnum, Clone, Copy, Debug, Default)]
enum CliRomFormat {
    #[default]
    Str,
    Edges,
    Alts,
    Lattice,
}

impl From<CliRomFormat> for RomFormat {
    fn from(cli_format: CliRomFormat) -> Self {
        match cli_format {
            CliRomFormat::Str => RomFormat::Str,
            CliRomFormat::Edges => RomFormat::Edges,
            CliRomFormat::Alts => RomFormat::Alts,
            CliRomFormat::Lattice => RomFormat::Lattice,
        }
    }
}

#[derive(Error, Debug)]
enum UromanError {
    #[error("Failed to open input file '{path}': {source}")]
    InputFileOpen { path: PathBuf, source: io::Error },

    #[error("Failed to create output file '{path}': {source}")]
    OutputFileCreate { path: PathBuf, source: io::Error },

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("REPL error: {0}")]
    Repl(#[from] ReadlineError),

    #[error("Romanization failed: {0}")]
    Romanization(#[from] RomanizationError),
}

#[derive(Parser, Debug)]
#[command(author, version)]
struct Cli {
    /// Direct text input to be romanized.
    #[arg(value_name = "DIRECT_INPUT")]
    direct_input: Vec<String>,

    /// Input file path (default: stdin).
    #[arg(short, long, value_name = "FILE")]
    input_filename: Option<PathBuf>,

    /// Output file path (default: stdout).
    #[arg(short, long, value_name = "FILE")]
    output_filename: Option<PathBuf>,

    /// [ISO 639-3 language code](https://www.loc.gov/standards/iso639-2/php/code_list.php) (e.g., 'eng').
    #[arg(short = 'l', long)]
    lcode: Option<String>,

    /// Output format of romanization. 'edges' provides offsets.
    #[arg(short = 'f', long, value_enum, default_value_t = CliRomFormat::default())]
    rom_format: CliRomFormat,

    /// Limit uroman to the first n lines of a file.
    #[arg(long)]
    max_lines: Option<usize>,

    /// Decodes Unicode escape notation, e.g., \\u03B4 to δ.
    #[arg(short = 'd', long, action = clap::ArgAction::SetTrue)]
    decode_unicode: bool,

    /// Enable parallel file processing.
    #[arg(short = 'p', long = "use-parallel", action = clap::ArgAction::SetTrue)]
    use_parallel: bool,

    /// Run and display a few samples.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    sample: bool,

    /// Suppress progress indicators.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    silent: bool,
}

fn main() {
    if let Err(err) = run() {
        if let UromanError::Io(e) = &err
            && e.kind() == io::ErrorKind::BrokenPipe
        {
            return;
        }

        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), UromanError> {
    let cli = Cli::parse();
    let uroman = Uroman::new();

    if cli.direct_input.is_empty()
        && cli.input_filename.is_none()
        && !cli.sample
        && std::io::stdin().is_terminal()
    {
        run_repl(&uroman, &cli)?;
        return Ok(());
    }

    if cli.sample
        && cli.direct_input.is_empty()
        && cli.input_filename.is_none()
        && cli.output_filename.is_none()
        && !cli.silent
    {
        show_samples(&uroman)?;
        return Ok(());
    }

    let mut writer = get_writer(&cli.output_filename)?;

    if !cli.direct_input.is_empty() {
        process_direct_input(&uroman, &cli, &mut writer)?;
    }

    if cli.input_filename.is_some() || cli.direct_input.is_empty() {
        process_stream(&uroman, &cli, &mut writer)?;
    }

    writer.flush()?;

    if cli.sample {
        println!(
            "Note: The --sample option was ignored because input was provided via other flags."
        );
    }

    Ok(())
}

fn process_direct_input(
    uroman: &Uroman,
    cli: &Cli,
    writer: &mut dyn Write,
) -> Result<(), UromanError> {
    let rom_format = Some(cli.rom_format.into());
    let lcode = cli.lcode.as_deref();
    for s in &cli.direct_input {
        let result = if !cli.decode_unicode {
            uroman.romanize_with_format(s, lcode, rom_format)
        } else {
            uroman.romanize_escaped_with_format(s, lcode, rom_format)
        };
        writeln!(writer, "{}", result.to_string()?)?;
    }
    Ok(())
}

fn process_stream(uroman: &Uroman, cli: &Cli, writer: &mut dyn Write) -> Result<(), UromanError> {
    let reader = get_reader(&cli.input_filename)?;

    if cli.use_parallel {
        uroman.romanize_file_parallel(
            reader,
            writer,
            cli.lcode.as_deref(),
            cli.rom_format.into(),
            cli.max_lines,
            cli.decode_unicode,
            cli.silent,
        )?;
    } else {
        uroman.romanize_file(
            reader,
            writer,
            cli.lcode.as_deref(),
            cli.rom_format.into(),
            cli.max_lines,
            cli.decode_unicode,
            cli.silent,
        )?;
    }
    Ok(())
}

fn get_reader(path: &Option<PathBuf>) -> Result<Box<dyn BufRead>, UromanError> {
    match path {
        Some(p) => {
            let file = fs::File::open(p).map_err(|e| UromanError::InputFileOpen {
                path: p.clone(),
                source: e,
            })?;
            Ok(Box::new(BufReader::new(file)))
        }
        None => Ok(Box::new(BufReader::new(io::stdin()))),
    }
}

fn get_writer(path: &Option<PathBuf>) -> Result<Box<dyn Write>, UromanError> {
    match path {
        Some(p) => {
            let file = fs::File::create(p).map_err(|e| UromanError::OutputFileCreate {
                path: p.clone(),
                source: e,
            })?;
            Ok(Box::new(BufWriter::new(file)))
        }
        None => Ok(Box::new(BufWriter::new(io::stdout()))),
    }
}

fn run_repl(uroman: &Uroman, cli: &Cli) -> Result<(), UromanError> {
    let mut rl = DefaultEditor::new()?;

    let history_path = || -> Option<std::path::PathBuf> {
        let mut path = dirs::cache_dir()?;
        path.push("uroman-rs");
        std::fs::create_dir_all(&path).ok()?;
        path.push("history.txt");
        Some(path)
    };

    if let Some(path) = history_path()
        && rl.load_history(&path).is_err()
    {}

    let lcode = cli.lcode.as_deref();

    loop {
        let readline = rl.readline(">> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(&line)?;

                if line.trim() == ":exit" || line.trim() == ":quit" {
                    break;
                }

                if line.trim().is_empty() {
                    continue;
                }

                match uroman
                    .romanize_with_format(&line, lcode, Some(cli.rom_format.into()))
                    .to_string()
                {
                    Ok(output) => println!("{output}"),
                    Err(e) => eprintln!("Error formatting output: {e}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted. To exit, press Ctrl-D or type :exit.");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Exiting.");
                break;
            }
            Err(err) => {
                eprintln!("REPL Error: {err}");
                break;
            }
        }
    }

    if let Some(path) = history_path()
        && let Err(err) = rl.save_history(&path)
    {
        eprintln!("Warning: could not save history to {path:?}: {err}");
    }

    Ok(())
}

fn show_samples(uroman: &Uroman) -> Result<(), UromanError> {
    println!("Running sample conversions with uroman-rs:");
    println!("---------------------------------------");

    let samples = [
        ("jpn", "一兆二千万四十二えん ほしい！"),
        ("amh", "ሰላም ልዑል!"),
        ("ara", "مرحبا بالعالم"),
        ("ell", "Καλημέρα, κόσμε."),
        ("heb", "שלום עולם"),
        ("hin", "नमस्ते दुनिया"),
        ("hye", "Բարև աշխարհ"),
        ("kor", "안녕하세요 세계"),
        ("rus", "Привет, мир! Как дела?"),
        ("tai", "สวัสดีชาวโลก"),
        ("ukr", "Привіт, світе!"),
        ("zho", "你好，世界！谢谢。"),
        ("", "¡Hola! ¿Cómo estás?"),
        ("", "မင်္ဂလာပါ"),
        ("", "ལྷ་ས་གྲོང་ཁྱེར"),
        ("", "ສະບາຍດີ"),
        ("", "ᚑᚌᚐᚋ ᚛ᚅᚐᚋᚓ᚜"),
        ("", "ᐊᕐᕌᒍᒥ ᓄᑖᒥ ᖁᕕᐊᓱᒋᑦ"),
        ("", "გამარჯობა"),
        ("", "ಧನ್ಯವಾದಗಳು"),
        ("", "ⴰⵎⵢⴰ ⵉⵊⵊⴻⵏ ⵙⵉⵏ"),
        ("", "⠓⠑⠇⠇⠕ ⠺⠕⠗⠇⠙"),
        ("", "𓊪𓏏𓍯𓃭𓐝𓇌𓋴"),
        ("", "ᚺᚨᛚᛚᛟ ᚹᛟᚱᛚᛞ"),
        ("", "ꦧꦱꦗꦮ"),
        ("", "Tôi yêu tiếng Việt!"),
        ("", "✨ユーロマン✨（ウロマン）"),
    ];

    let max_width = 29;
    let mut total_duration_ns: u128 = 0;

    for (lang_code, text) in samples.iter() {
        let start = time::Instant::now();
        let romanized = uroman
            .romanize_string::<rom_format::Str>(text, Some(lang_code))
            .to_string();
        let duration = start.elapsed();
        total_duration_ns += duration.as_nanos();

        let current_width = UnicodeWidthStr::width(*text);
        let padding = " ".repeat(max_width - current_width);
        if lang_code.is_empty() {
            println!("      {text}{padding} -> {romanized}");
        } else {
            println!("[{lang_code}] {text}{padding} -> {romanized}");
        }
    }

    println!("---------------------------------------");

    if let Some(avg_duration_ns) = total_duration_ns.checked_div(samples.len() as u128) {
        let avg_duration_us = avg_duration_ns as f64 / 1_000.0;
        let avg_duration_ms = avg_duration_us / 1_000.0;

        println!(
            "Avg. processing time: {avg_duration_ms:.3} ms ({avg_duration_us:.1} μs) per sample"
        );
    }

    Ok(())
}
