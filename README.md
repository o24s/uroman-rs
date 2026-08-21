<div align="center">
  <h1>uroman-rs</h1>
  <p>
    A self-contained Rust reimplementation of the <code>uroman</code> universal romanizer.
  </p>
  <p>
    <a href="https://crates.io/crates/uroman"><img src="https://img.shields.io/crates/v/uroman.svg" alt="Crates.io"></a>
    <a href="https://github.com/stellanomia/uroman-rs/actions/workflows/rust.yml"><img src="https://github.com/stellanomia/uroman-rs/actions/workflows/rust.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg" alt="License: Apache-2.0"></a>
  </p>
</div>

## Overview

`uroman-rs` is a rewrite of the original `uroman` (Universal Romanizer) in Rust. It converts text in non-Latin writing systems to the Latin alphabet.

It reproduces the original's output rather than improving on it, so its romanization logic and its limitations are those of the original, apart from the cases listed under [Differences from the original](#differences-from-the-original). Before using it, read the original authors' documentation on [Reversibility](https://github.com/isi-nlp/uroman?tab=readme-ov-file#reversibility) and [Known Limitations](https://github.com/isi-nlp/uroman?tab=readme-ov-file#limitations), which apply here too.

`uroman-rs` is licensed under the Apache License 2.0 and includes the original's license notice, as required. See the [License section](#license).

## Features

- Produces byte-identical output to `uroman.py` on the original's `multi-script.txt` test file.
- Dictionaries are compiled at build time into a zero-copy [`rkyv`](https://github.com/rkyv/rkyv) archive that is read in place, so nothing is parsed at startup. The CLI starts in about a millisecond, and `Uroman::new()` copies a reference.
- About 188x faster than the Python implementation on that test file, mostly because of startup. (See [Benchmark](#benchmark))
- Does not raise on the inputs that make the original raise, and supplies the vowel where the original emits a bare apostrophe. (See [Differences from the original](#differences-from-the-original))
- Output formats: plain string (`str`) and JSON (`edges`, `alts`, `lattice`).
- No runtime outside the binary; builds to a single executable.
- Usable as a command-line tool or as a library.

## Installation

The `uroman-rs` project is available as a crate named uroman. You can use it both as a command-line tool and as a library in your Rust projects.

### As a Command-Line Tool

To install the `uroman-rs` command-line tool, run the following:

```bash
cargo install uroman -F cli
```

This will install the executable as `uroman-rs` on your system.

### As a Library

Add `uroman-rs` to your project's Cargo.toml.

```bash
cargo add uroman
```

## Usage

### Command-Line Interface (CLI)

`uroman-rs` can be used directly from your terminal.

**Show sample conversions:**
See examples of how various scripts are romanized.

```bash
uroman-rs --sample
```

**View all options:**

Display the help message for a full list of commands and flags.
```bash
uroman-rs --help
```

**Use in REPL mode:**

Run `uroman-rs` without any arguments to process input line by line. Press `Ctrl+D` to exit.

```bash
$ uroman-rs
>> こんにちは、世界！
konnichiha, shijie!
>> ᚺᚨᛚᛚᛟ ᚹᛟᚱᛚᛞ
hallo world
>> (Ctrl+D)
```


### Library

```rust
// Uroman::new() is infallible and does not return a `Result`.
let uroman = Uroman::new();

let romanized_string/*: String*/ = uroman.romanize_string::<rom_format::Str>(
    "✨ユーロマン✨",
    Some("jpn"),
).to_string();

assert_eq!(romanized_string, "✨yuuroman✨");
println!("{romanized_string}");
```
For more advanced examples, please see the examples/ directory.

#### Memory-mapped dictionaries (optional)

By default the dictionary archive is embedded in the binary. With the `mmap`
feature you can instead load it from an external file via memory mapping,
e.g. to share one dictionary file between many processes:

```toml
uroman = { version = "...", features = ["mmap"] }
```

```rust,ignore
// Write the archive out once (e.g. at install time):
std::fs::write("uroman.dicts.rkyv", uroman::dictionary_archive_bytes())?;

// Later, load it with mmap (the archive is validated before use):
let uroman = uroman::Uroman::from_dictionary_file("uroman.dicts.rkyv")?;
```

## Benchmark

Performance was measured against the original Python implementation using [`hyperfine`](https://github.com/sharkdp/hyperfine). Both produce byte-identical output on the test file.

- Test File: `multi-script.txt` from the original `uroman` repository (32 lines).
- Environment: Intel Core i7-14700, Ubuntu 26.04, `uroman.py` 1.3.1.1 via `uv run`.

| Implementation             | Mean Time (± σ)  | Relative    |
|----------------------------|------------------|-------------|
| `uroman-rs`                | 7.5 ms ± 1.1 ms  | 188x faster |
| `uroman.py` (via `uv run`) | 1407 ms ± 9 ms   | Baseline    |

On a file this size the figure is dominated by start-up, which is where the
gap is widest: romanizing a single line takes `uroman-rs` 0.87 ms against
1.34 s for `uroman.py`, because the dictionaries are read from a
precompiled archive rather than parsed on every run.

For sustained throughput on large inputs the ratio is smaller and depends on
the corpus, so no single number is quoted here. Note that `uroman.py` caches
romanizations per line, so a benchmark built by repeating the same lines
measures that cache rather than throughput. `uroman-rs` additionally offers
`--use-parallel` to process lines across cores.

## Differences from the original

`uroman-rs` departs from the original in two places: one where the original raises an exception, and one where it drops a vowel.

### Incomplete percentage patterns

The original script raises on inputs with an incomplete fractional pattern such as `"百分之"` ("percent of ..."), because it expects a number to follow and does not handle its absence. This has been reported upstream (see [isi-nlp/uroman#16](https://github.com/isi-nlp/uroman/issues/16)).

```sh
$ uv run uroman.py "百分之多少"
Traceback (most recent call last):
  ...
AttributeError: 'NoneType' object has no attribute 'value'
```

`uroman-rs` falls back to syllable-by-syllable pinyin:

```sh
$ uroman-rs "百分之多少"
baifenzhiduoshao
```

### Tibetan letter -A (U+0F60) standing alone

Romanizing `འ` (U+0F60, TIBETAN LETTER -A) on its own, the original produces a bare apostrophe:

```sh
$ uv run uroman.py "འ"
'
```

The syllable is pronounced. Tibetan is an abugida, so a root letter with no vowel sign carries the inherent vowel /a/, and an output with no vowel in it is not much use to a romanizer. `uroman-rs` supplies the vowel:

```sh
$ uroman-rs "འ"
'a
```

The apostrophe is kept because that is how [Wylie](https://texts.mandala.library.virginia.edu/text/thl-extended-wylie-transliteration-scheme) and [ALA-LC](https://www.loc.gov/catdir/cpso/romanization/tibetan.pdf) write this letter. The phonetic [THL Simplified](https://en.wikipedia.org/wiki/THL_Simplified_Phonetic_Transcription) system would write just `a`, which would be equally defensible; the claim here is only that the bare `'` is not.

The same applies when `འ` carries a vowel sign, where the original also drops the letter: `འེ` → `'e`, `འུ` → `'u`, `འོ` → `'o`, against `e`, `u`, `o`. `འི` is an exception, romanized `i` as in the original, through a rule this project added to `data/romanization-table.txt`.

Only `འ` as a root letter is affected. Its other uses — silent prefix, silent suffix, and the source of the nasalization in words such as `དགེ་འདུན` — are untouched, and the two implementations agree there (`འགྲོ` → `'gro`, `བའི` → `ba'i`). On the Tibetan line of the original's own `multi-script.txt` they produce identical output.

## License

This project is licensed under the Apache License, Version 2.0.

### Acknowledgements

`uroman-rs` is a Rust implementation of the original `uroman` software by Ulf Hermjakob. As such, it is a derivative work and includes the original license notice in the `NOTICE` file.

Please be aware that any academic publication of projects using `uroman-rs` should acknowledge the use of the original `uroman` software as specified in its license. For details, please see the `NOTICE` file.
