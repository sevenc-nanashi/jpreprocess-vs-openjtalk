use open_jtalk::{text2mecab, JpCommon, ManagedResource, Mecab, Njd};
use serde::Serialize;
use std::panic;
use std::str::FromStr;
use std::time::Instant;

static DICT_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/open_jtalk_dic_utf_8-1.11"
);

static OJT_RESOURCES: std::sync::LazyLock<std::sync::Mutex<Resources>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(Resources {
            mecab: ManagedResource::initialize(),
            njd: ManagedResource::initialize(),
            jpcommon: ManagedResource::initialize(),
        })
    });

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Difference {
    Light,
    Fatal,
}

// ---- JSON output types ----

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Results {
    generated_at: String,
    commit: String,
    totals: Stats,
    files: Vec<FileResult>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    total: usize,
    characters: usize,
    matches: usize,
    light_mismatches: usize,
    fatal_mismatches: usize,
    jp_errors: usize,
    ojt_errors: usize,
    openjtalk_extraction_duration_ms: f64,
    openjtalk_throughput_chars_per_second: f64,
    jpreprocess_extraction_duration_ms: f64,
    jpreprocess_throughput_chars_per_second: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileResult {
    file: String,
    stats: Stats,
    entries: Vec<Entry>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Entry {
    #[serde(rename = "match")]
    Match(MatchEntry),
    #[serde(rename = "light")]
    Light(MismatchEntry),
    #[serde(rename = "fatal")]
    Fatal(MismatchEntry),
    #[serde(rename = "jp_error")]
    JpError(ErrorEntry),
    #[serde(rename = "ojt_error")]
    OjtError(ErrorEntry),
    #[serde(rename = "both_error")]
    BothError(ErrorEntry),
    #[serde(rename = "jp_panic")]
    JpPanic(ErrorEntry),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchEntry {
    index: usize,
    original: String,
    openjtalk: Vec<Phoneme>,
    jpreprocess: Vec<Phoneme>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MismatchEntry {
    index: usize,
    original: String,
    openjtalk: Vec<Phoneme>,
    jpreprocess: Vec<Phoneme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length_mismatch: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorEntry {
    index: usize,
    original: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    openjtalk_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jpreprocess_error: Option<String>,
}

#[derive(Serialize)]
struct Phoneme {
    value: String,
    diff: DiffKind,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum DiffKind {
    None,
    Light,
    Fatal,
}

// Compute per-phoneme diffs between two phoneme sequences.
// For positions beyond the shorter list, the extra phonemes are Fatal.
fn phonemes_with_diff(primary: &[String], other: &[String]) -> Vec<Phoneme> {
    primary
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let diff = if i >= other.len() {
                DiffKind::Fatal
            } else if p == &other[i] {
                DiffKind::None
            } else if p.eq_ignore_ascii_case(&other[i]) {
                DiffKind::Light
            } else {
                DiffKind::Fatal
            };
            Phoneme {
                value: p.clone(),
                diff,
            }
        })
        .collect()
}

fn throughput_chars_per_second(characters: usize, extraction_duration_ms: f64) -> f64 {
    if extraction_duration_ms == 0.0 {
        0.0
    } else {
        characters as f64 / (extraction_duration_ms / 1000.0)
    }
}

fn main() -> anyhow::Result<()> {
    {
        let mut resources = OJT_RESOURCES.lock().unwrap();
        resources.mecab.load(DICT_DIR)?;
    }

    // Parse --json <path> from args
    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let mut json_path: Option<String> = None;
    let mut file_paths: Vec<std::path::PathBuf> = vec![];
    {
        let mut iter = raw_args.into_iter();
        while let Some(arg) = iter.next() {
            if arg == "--json" {
                json_path = iter.next();
            } else {
                file_paths.push(std::path::PathBuf::from(arg));
            }
        }
    }

    let mut total_matches = 0usize;
    let mut total_light_mismatches = 0usize;
    let mut total_fatal_mismatches = 0usize;
    let mut total_jp_errors = 0usize;
    let mut total_ojt_errors = 0usize;
    let mut total_characters = 0usize;
    let mut total_openjtalk_extraction_duration_ms = 0.0f64;
    let mut total_jpreprocess_extraction_duration_ms = 0.0f64;

    let mut file_stats_display = vec![];
    let mut all_file_results: Vec<FileResult> = vec![];

    let mut jp = jpreprocess::JPreprocess::with_dictionaries(
        jpreprocess::SystemDictionaryConfig::Bundled(
            jpreprocess::kind::JPreprocessDictionaryKind::NaistJdic,
        )
        .load()?,
        None,
    );

    for file in &file_paths {
        let text = std::fs::read_to_string(file)?;
        let sentences = lazy_regex::regex!("[。「」]")
            .split(&text)
            .map(|s| lazy_regex::regex_replace_all!(r"\s+", s, ""))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let sentences_size = sentences.len();
        let characters = sentences.iter().map(|s| s.chars().count()).sum::<usize>();
        let mut openjtalk_extraction_duration_ms = 0.0f64;
        let mut jpreprocess_extraction_duration_ms = 0.0f64;
        let mut matches = 0usize;
        let mut light_mismatches = 0usize;
        let mut fatal_mismatches = 0usize;
        let mut jp_errors = 0usize;
        let mut ojt_errors = 0usize;
        let mut entries: Vec<Entry> = vec![];

        for (sentence_i, sentence) in sentences.iter().enumerate() {
            let prefix = format!(
                "[{} : {} / {}]: ",
                file.file_name().unwrap().to_string_lossy(),
                sentence_i + 1,
                sentences_size
            );
            let openjtalk_extraction_started = Instant::now();
            let ojt_labels = extract_fullcontext(sentence);
            openjtalk_extraction_duration_ms +=
                openjtalk_extraction_started.elapsed().as_secs_f64() * 1000.0;

            let jpreprocess_extraction_started = Instant::now();
            let jp_labels = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                jp.extract_fullcontext(sentence)
                    .map_err(anyhow::Error::from)
            }))
            .map_err(|e| anyhow::anyhow!("panicked! {:?}", e.downcast_ref::<String>()))
            .and_then(|r| r);
            jpreprocess_extraction_duration_ms +=
                jpreprocess_extraction_started.elapsed().as_secs_f64() * 1000.0;

            let (ojt_labels, jp_labels) = match (ojt_labels, jp_labels) {
                (Ok(ojt_labels), Ok(jp_labels)) => (ojt_labels, jp_labels),
                (r1, r2) => {
                    let ojt_err = r1.as_ref().err().map(|e| e.to_string());
                    let jp_err = r2.as_ref().err().map(|e| e.to_string());
                    if r1.is_err() {
                        ojt_errors += 1;
                    }
                    if r2.is_err() {
                        jp_errors += 1;
                        // 念のためリセット
                        jp = jpreprocess::JPreprocess::with_dictionaries(
                            jpreprocess::SystemDictionaryConfig::Bundled(
                                jpreprocess::kind::JPreprocessDictionaryKind::NaistJdic,
                            )
                            .load()?,
                            None,
                        );
                    }
                    let kind = if r1.is_err() && r2.is_err() {
                        "Both"
                    } else if r1.is_err() {
                        "OpenJTalk"
                    } else if r2.as_ref().unwrap_err().to_string().contains("panicked!") {
                        "JPreprocess (panicked)"
                    } else {
                        "JPreprocess"
                    };
                    println!("{} \x1b[35m{} Error:\x1b[0m", prefix, kind);
                    println!("     Original: {}", sentence);
                    println!("    OpenJTalk: {:?}", r1.map(|_| ()));
                    println!("  JPreprocess: {:?}", r2.map(|_| ()));

                    let error_entry = ErrorEntry {
                        index: sentence_i,
                        original: sentence.to_string(),
                        openjtalk_error: ojt_err,
                        jpreprocess_error: jp_err,
                    };
                    entries.push(match kind {
                        "Both" => Entry::BothError(error_entry),
                        "OpenJTalk" => Entry::OjtError(error_entry),
                        "JPreprocess (panicked)" => Entry::JpPanic(error_entry),
                        _ => Entry::JpError(error_entry),
                    });
                    continue;
                }
            };
            let ojt_phonemes = ojt_labels
                .iter()
                .filter_map(|l| l.phoneme.c.clone())
                .collect::<Vec<_>>();
            let jp_phonemes = jp_labels
                .iter()
                .filter_map(|l| l.phoneme.c.clone())
                .collect::<Vec<_>>();
            if ojt_phonemes == jp_phonemes {
                matches += 1;
                let phonemes = phonemes_with_diff(&ojt_phonemes, &jp_phonemes);
                let phonemes_jp = phonemes_with_diff(&jp_phonemes, &ojt_phonemes);
                entries.push(Entry::Match(MatchEntry {
                    index: sentence_i,
                    original: sentence.to_string(),
                    openjtalk: phonemes,
                    jpreprocess: phonemes_jp,
                }));
            } else {
                let differences = if ojt_phonemes.len() == jp_phonemes.len() {
                    let differences = ojt_phonemes
                        .iter()
                        .zip(jp_phonemes.iter())
                        .map(|(o, j)| {
                            if o == j {
                                None
                            } else if o.eq_ignore_ascii_case(j) {
                                Some(Difference::Light)
                            } else {
                                Some(Difference::Fatal)
                            }
                        })
                        .collect::<Vec<_>>();

                    if differences.is_empty() {
                        None
                    } else {
                        Some(differences)
                    }
                } else {
                    None
                };
                if let Some(differences) = differences {
                    let mut ojt_buffer = String::new();
                    let mut jp_buffer = String::new();

                    for (ojt_phoneme, jp_phonemes, difference) in itertools::izip!(
                        ojt_phonemes.iter(),
                        jp_phonemes.iter(),
                        differences.iter()
                    ) {
                        let length = ojt_phoneme.len().max(jp_phonemes.len());
                        match difference {
                            None => {
                                ojt_buffer.push_str(&format!(
                                    "{:>width$}",
                                    ojt_phoneme,
                                    width = length
                                ));
                                jp_buffer.push_str(&format!(
                                    "{:>width$}",
                                    jp_phonemes,
                                    width = length
                                ));
                            }
                            Some(Difference::Light) => {
                                ojt_buffer.push_str(&format!(
                                    "\x1b[33m{:>width$}\x1b[0m",
                                    ojt_phoneme,
                                    width = length
                                ));
                                jp_buffer.push_str(&format!(
                                    "\x1b[33m{:>width$}\x1b[0m",
                                    jp_phonemes,
                                    width = length
                                ));
                            }
                            Some(Difference::Fatal) => {
                                ojt_buffer.push_str(&format!(
                                    "\x1b[31m{:>width$}\x1b[0m",
                                    ojt_phoneme,
                                    width = length
                                ));
                                jp_buffer.push_str(&format!(
                                    "\x1b[31m{:>width$}\x1b[0m",
                                    jp_phonemes,
                                    width = length
                                ));
                            }
                        }

                        ojt_buffer.push(' ');
                        jp_buffer.push(' ');
                    }

                    let is_fatal = differences.iter().any(|d| d == &Some(Difference::Fatal));
                    if is_fatal {
                        println!("{}\x1b[31mFatal mismatch:\x1b[0m", prefix,);
                        fatal_mismatches += 1;
                    } else {
                        println!("{}\x1b[33mLight mismatch:\x1b[0m", prefix,);
                        light_mismatches += 1;
                    }
                    println!("     Original: {}", sentence);
                    println!("    OpenJTalk: {}", ojt_buffer.trim());
                    println!("  JPreprocess: {}", jp_buffer.trim());

                    let phonemes_ojt = phonemes_with_diff(&ojt_phonemes, &jp_phonemes);
                    let phonemes_jp = phonemes_with_diff(&jp_phonemes, &ojt_phonemes);
                    let entry = MismatchEntry {
                        index: sentence_i,
                        original: sentence.to_string(),
                        openjtalk: phonemes_ojt,
                        jpreprocess: phonemes_jp,
                        length_mismatch: None,
                    };
                    entries.push(if is_fatal {
                        Entry::Fatal(entry)
                    } else {
                        Entry::Light(entry)
                    });
                } else {
                    println!(
                        "{}\x1b[31mFatal mismatch: (length mismatch: OpenJTalk: {}, JPreprocess: {})\x1b[0m",
                        prefix,
                        ojt_phonemes.len(),
                        jp_phonemes.len()
                    );

                    let mut ojt_light_mismatch_left = ojt_phonemes.len();
                    let mut jp_light_mismatch_left = jp_phonemes.len();
                    let mut ojt_fatal_mismatch_left = ojt_phonemes.len();
                    let mut jp_fatal_mismatch_left = jp_phonemes.len();
                    for (i, (ojt_phoneme, jp_phoneme)) in
                        itertools::izip!(ojt_phonemes.iter(), jp_phonemes.iter()).enumerate()
                    {
                        if ojt_phoneme != jp_phoneme {
                            ojt_light_mismatch_left = i;
                            jp_light_mismatch_left = i;
                        }
                        if !ojt_phoneme.eq_ignore_ascii_case(jp_phoneme) {
                            ojt_fatal_mismatch_left = i;
                            jp_fatal_mismatch_left = i;
                            break;
                        }
                    }
                    let mut ojt_light_mismatch_right = ojt_phonemes.len();
                    let mut jp_light_mismatch_right = jp_phonemes.len();
                    let mut ojt_fatal_mismatch_right = ojt_phonemes.len();
                    let mut jp_fatal_mismatch_right = jp_phonemes.len();
                    for (i, (ojt_phoneme, jp_phoneme)) in
                        itertools::izip!(ojt_phonemes.iter().rev(), jp_phonemes.iter().rev())
                            .enumerate()
                    {
                        if ojt_phoneme != jp_phoneme {
                            ojt_light_mismatch_right = ojt_phonemes.len() - i;
                            jp_light_mismatch_right = jp_phonemes.len() - i;
                        }
                        if !ojt_phoneme.eq_ignore_ascii_case(jp_phoneme) {
                            ojt_fatal_mismatch_right = ojt_phonemes.len() - i;
                            jp_fatal_mismatch_right = jp_phonemes.len() - i;
                            break;
                        }
                    }

                    let mut ojt_buffer = String::new();
                    let mut jp_buffer = String::new();

                    for (i, ojt_phoneme) in ojt_phonemes.iter().enumerate() {
                        let length = ojt_phoneme.len();
                        if i == ojt_light_mismatch_left {
                            ojt_buffer.push_str("\x1b[33m");
                        }
                        if i == ojt_fatal_mismatch_left {
                            ojt_buffer.push_str("\x1b[31m");
                        }

                        ojt_buffer.push_str(&format!("{:>width$}", ojt_phoneme, width = length));

                        if i == ojt_fatal_mismatch_right {
                            ojt_buffer.push_str("\x1b[33m");
                        }
                        if i == ojt_light_mismatch_right {
                            ojt_buffer.push_str("\x1b[0m");
                        }

                        ojt_buffer.push(' ');
                    }
                    for (i, jp_phoneme) in jp_phonemes.iter().enumerate() {
                        let length = jp_phoneme.len();
                        if i == jp_light_mismatch_left {
                            jp_buffer.push_str("\x1b[33m");
                        }
                        if i == jp_fatal_mismatch_left {
                            jp_buffer.push_str("\x1b[31m");
                        }

                        jp_buffer.push_str(&format!("{:>width$}", jp_phoneme, width = length));

                        if i == jp_fatal_mismatch_right {
                            jp_buffer.push_str("\x1b[33m");
                        }
                        if i == jp_light_mismatch_right {
                            jp_buffer.push_str("\x1b[0m");
                        }

                        jp_buffer.push(' ');
                    }
                    ojt_buffer = ojt_buffer.trim().to_string();
                    jp_buffer = jp_buffer.trim().to_string();
                    ojt_buffer.push_str("\x1b[0m");
                    jp_buffer.push_str("\x1b[0m");

                    println!("     Original: {}", sentence);
                    fatal_mismatches += 1;
                    println!("    OpenJTalk: {}", ojt_buffer);
                    println!("  JPreprocess: {}", jp_buffer);

                    let phonemes_ojt = phonemes_with_diff(&ojt_phonemes, &jp_phonemes);
                    let phonemes_jp = phonemes_with_diff(&jp_phonemes, &ojt_phonemes);
                    entries.push(Entry::Fatal(MismatchEntry {
                        index: sentence_i,
                        original: sentence.to_string(),
                        openjtalk: phonemes_ojt,
                        jpreprocess: phonemes_jp,
                        length_mismatch: Some(true),
                    }));
                }
            }
        }

        let openjtalk_throughput_chars_per_second =
            throughput_chars_per_second(characters, openjtalk_extraction_duration_ms);
        let jpreprocess_throughput_chars_per_second =
            throughput_chars_per_second(characters, jpreprocess_extraction_duration_ms);
        file_stats_display.push(format!(
            "{}: \x1b[32m{} matches\x1b[0m, \x1b[33m{} light mismatches\x1b[0m, \x1b[31m{} fatal mismatches\x1b[0m, \x1b[35m{} jpreprocess errors\x1b[0m, \x1b[35m{} open_jtalk errors\x1b[0m, OpenJTalk: {:.0} chars/s ({:.2} ms), JPreprocess: {:.0} chars/s ({:.2} ms), {} chars",
            file.file_name().unwrap().to_string_lossy(),
            matches,
            light_mismatches,
            fatal_mismatches,
            jp_errors,
            ojt_errors,
            openjtalk_throughput_chars_per_second,
            openjtalk_extraction_duration_ms,
            jpreprocess_throughput_chars_per_second,
            jpreprocess_extraction_duration_ms,
            characters
        ));

        all_file_results.push(FileResult {
            file: file.file_name().unwrap().to_string_lossy().to_string(),
            stats: Stats {
                total: sentences_size,
                characters,
                matches,
                light_mismatches,
                fatal_mismatches,
                jp_errors,
                ojt_errors,
                openjtalk_extraction_duration_ms,
                openjtalk_throughput_chars_per_second,
                jpreprocess_extraction_duration_ms,
                jpreprocess_throughput_chars_per_second,
            },
            entries,
        });

        total_matches += matches;
        total_light_mismatches += light_mismatches;
        total_fatal_mismatches += fatal_mismatches;
        total_jp_errors += jp_errors;
        total_ojt_errors += ojt_errors;
        total_characters += characters;
        total_openjtalk_extraction_duration_ms += openjtalk_extraction_duration_ms;
        total_jpreprocess_extraction_duration_ms += jpreprocess_extraction_duration_ms;
    }

    for file_stat in file_stats_display {
        println!("{}", file_stat);
    }

    println!();
    let total_openjtalk_throughput_chars_per_second =
        throughput_chars_per_second(total_characters, total_openjtalk_extraction_duration_ms);
    let total_jpreprocess_throughput_chars_per_second =
        throughput_chars_per_second(total_characters, total_jpreprocess_extraction_duration_ms);
    println!(
        "Total: \x1b[32m{} matches\x1b[0m, \x1b[33m{} light mismatches\x1b[0m, \x1b[31m{} fatal mismatches\x1b[0m, \x1b[35m{} jpreprocess errors\x1b[0m, \x1b[35m{} open_jtalk errors\x1b[0m, OpenJTalk: {:.0} chars/s ({:.2} ms), JPreprocess: {:.0} chars/s ({:.2} ms), {} chars",
        total_matches,
        total_light_mismatches,
        total_fatal_mismatches,
        total_jp_errors,
        total_ojt_errors,
        total_openjtalk_throughput_chars_per_second,
        total_openjtalk_extraction_duration_ms,
        total_jpreprocess_throughput_chars_per_second,
        total_jpreprocess_extraction_duration_ms,
        total_characters
    );

    if let Some(path) = json_path {
        let total_sentences: usize = all_file_results.iter().map(|f| f.stats.total).sum();
        let results = Results {
            generated_at: chrono::Local::now().to_rfc3339(),
            commit: std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local".to_string()),
            totals: Stats {
                total: total_sentences,
                characters: total_characters,
                matches: total_matches,
                light_mismatches: total_light_mismatches,
                fatal_mismatches: total_fatal_mismatches,
                jp_errors: total_jp_errors,
                ojt_errors: total_ojt_errors,
                openjtalk_extraction_duration_ms: total_openjtalk_extraction_duration_ms,
                openjtalk_throughput_chars_per_second: total_openjtalk_throughput_chars_per_second,
                jpreprocess_extraction_duration_ms: total_jpreprocess_extraction_duration_ms,
                jpreprocess_throughput_chars_per_second:
                    total_jpreprocess_throughput_chars_per_second,
            },
            files: all_file_results,
        };
        let json = serde_json::to_string(&results)?;
        std::fs::write(&path, json)?;
        eprintln!("JSON written to {}", path);
    }

    Ok(())
}

fn extract_fullcontext(text: &str) -> anyhow::Result<Vec<jlabel::Label>> {
    let Resources {
        mecab,
        njd,
        jpcommon,
    } = &mut *OJT_RESOURCES.lock().unwrap();

    jpcommon.refresh();
    njd.refresh();
    mecab.refresh();

    let mecab_text = text2mecab(text)?;
    if mecab.analysis(mecab_text) {
        njd.mecab2njd(
            mecab
                .get_feature()
                .ok_or(anyhow::anyhow!("mecab.get_feature()"))?,
            mecab.get_size(),
        );
        njd.set_pronunciation();
        njd.set_digit();
        njd.set_accent_phrase();
        njd.set_accent_type();
        njd.set_unvoiced_vowel();
        njd.set_long_vowel();
        jpcommon.njd2jpcommon(njd);
        jpcommon.make_label();
        jpcommon
            .get_label_feature_to_iter()
            .ok_or(anyhow::anyhow!("jpcommon.get_label_feature_to_iter()"))
            .map(|iter| iter.map(|s| jlabel::Label::from_str(s).unwrap()).collect())
            .map_err(Into::into)
    } else {
        anyhow::bail!("mecab.analysis() failed")
    }
}

struct Resources {
    mecab: ManagedResource<Mecab>,
    njd: ManagedResource<Njd>,
    jpcommon: ManagedResource<JpCommon>,
}

unsafe impl Send for Resources {}
unsafe impl Sync for Resources {}
