use open_jtalk::{ManagedResource, Mecab};
use serde::Serialize;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::panic;
use std::str::FromStr;
use std::time::Instant;

static DICT_DIR: &str = env!("OPENJTALK_DICT_DIR");

static OJT_RESOURCES: std::sync::LazyLock<std::sync::Mutex<Resources>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Resources::new()));

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    chunks: Vec<EntryChunk>,
}

#[derive(Serialize, Default)]
struct EntryCounts {
    matches: usize,
    light: usize,
    fatal: usize,
    errors: usize,
}

#[derive(Serialize)]
struct EntryChunk {
    path: String,
    counts: EntryCounts,
}

fn write_entry_chunk(
    directory: &std::path::Path,
    file_index: usize,
    chunk_index: usize,
    entries: &mut Vec<Entry>,
) -> anyhow::Result<EntryChunk> {
    // Avoid static servers treating .gz as HTTP content encoding.
    let path = format!("result-pages/{file_index}-{chunk_index}.bin");
    let mut counts = EntryCounts::default();
    for entry in entries.iter() {
        match entry {
            Entry::Match(_) => counts.matches += 1,
            Entry::Light(_) => counts.light += 1,
            Entry::Fatal(_) => counts.fatal += 1,
            _ => counts.errors += 1,
        }
    }
    let file = std::fs::File::create(directory.join(&path))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut writer = std::io::BufWriter::new(encoder);
    serde_json::to_writer(&mut writer, entries)?;
    writer.into_inner()?.finish()?;
    entries.clear();
    Ok(EntryChunk { path, counts })
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
    openjtalk_phrases: Vec<Phrase>,
    jpreprocess_phrases: Vec<Phrase>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MismatchEntry {
    index: usize,
    original: String,
    openjtalk: Vec<Phoneme>,
    jpreprocess: Vec<Phoneme>,
    openjtalk_phrases: Vec<Phrase>,
    jpreprocess_phrases: Vec<Phrase>,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
enum DiffKind {
    None,
    Light,
    Fatal,
}

#[derive(Debug, Serialize)]
struct Phrase {
    text: String,
    reading: String,
    diff: DiffKind,
}

#[derive(Debug)]
struct Analysis {
    labels: Vec<jlabel::Label>,
    phrases: Vec<Phrase>,
}

fn phrases_from_nodes(nodes: impl IntoIterator<Item = (String, String, bool)>) -> Vec<Phrase> {
    let mut phrases: Vec<Phrase> = Vec::new();
    for (text, reading, chained) in nodes {
        if chained && !phrases.is_empty() {
            let phrase = phrases.last_mut().unwrap();
            phrase.text.push_str(&text);
            phrase.reading.push_str(&reading);
        } else {
            phrases.push(Phrase {
                text,
                reading,
                diff: DiffKind::None,
            });
        }
    }
    phrases
}

fn reading_units(phrases: &[Phrase]) -> Vec<(char, bool, usize)> {
    let mut units: Vec<(char, bool, usize)> = Vec::new();
    for (index, phrase) in phrases.iter().enumerate() {
        for character in phrase.reading.chars() {
            if character == '’' {
                units
                    .last_mut()
                    .expect("Devoicing mark must follow a kana")
                    .1 = true;
            } else {
                units.push((character, false, index));
            }
        }
    }
    units
}

fn mark_phrase_diffs(primary: &mut [Phrase], other: &mut [Phrase]) {
    use similar::{capture_diff_slices, Algorithm, DiffTag};
    let primary_units = reading_units(primary);
    let other_units = reading_units(other);
    let primary_chars: Vec<_> = primary_units.iter().map(|unit| unit.0).collect();
    let other_chars: Vec<_> = other_units.iter().map(|unit| unit.0).collect();
    // Align readings independently of phrase boundaries, then compare devoicing on equal kana.
    for op in capture_diff_slices(Algorithm::Myers, &primary_chars, &other_chars) {
        let old = &primary_units[op.old_range()];
        let new = &other_units[op.new_range()];
        if op.tag() == DiffTag::Equal {
            for (left, right) in old.iter().zip(new) {
                if left.1 != right.1 {
                    primary[left.2].diff = primary[left.2].diff.max(DiffKind::Light);
                    other[right.2].diff = other[right.2].diff.max(DiffKind::Light);
                }
            }
        } else {
            for unit in old {
                primary[unit.2].diff = DiffKind::Fatal;
            }
            for unit in new {
                other[unit.2].diff = DiffKind::Fatal;
            }
        }
    }
}

// Compute per-phoneme diffs between two phoneme sequences using LCS-based diffing.
// Equal phonemes get DiffKind::None, case-insensitive matches get DiffKind::Light,
// and all other differences (including insertions/deletions) get DiffKind::Fatal.
fn phonemes_with_diff(primary: &[String], other: &[String]) -> Vec<Phoneme> {
    use similar::{capture_diff_slices, Algorithm, DiffOp};

    let ops = capture_diff_slices(Algorithm::Myers, primary, other);
    let mut result = Vec::new();

    for op in ops {
        match op {
            DiffOp::Equal { old_index, len, .. } => {
                for p in &primary[old_index..old_index + len] {
                    result.push(Phoneme {
                        value: p.clone(),
                        diff: DiffKind::None,
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for p in &primary[old_index..old_index + old_len] {
                    result.push(Phoneme {
                        value: p.clone(),
                        diff: DiffKind::Fatal,
                    });
                }
            }
            DiffOp::Insert { .. } => {
                // Other has extra phonemes not in primary; nothing to annotate here.
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let old_slice = &primary[old_index..old_index + old_len];
                let new_slice = &other[new_index..new_index + new_len];
                // Run a nested LCS on lowercased phonemes to identify case-insensitive
                // (Light) matches within the replaced block.
                let old_lower: Vec<String> = old_slice.iter().map(|s| s.to_lowercase()).collect();
                let new_lower: Vec<String> = new_slice.iter().map(|s| s.to_lowercase()).collect();
                for inner_op in capture_diff_slices(Algorithm::Myers, &old_lower, &new_lower) {
                    match inner_op {
                        DiffOp::Equal {
                            old_index: oi, len, ..
                        } => {
                            // Case-insensitive match within the replaced block.
                            for p in &old_slice[oi..oi + len] {
                                result.push(Phoneme {
                                    value: p.clone(),
                                    diff: DiffKind::Light,
                                });
                            }
                        }
                        DiffOp::Delete {
                            old_index: oi,
                            old_len: ol,
                            ..
                        } => {
                            for p in &old_slice[oi..oi + ol] {
                                result.push(Phoneme {
                                    value: p.clone(),
                                    diff: DiffKind::Fatal,
                                });
                            }
                        }
                        DiffOp::Insert { .. } => {
                            // Extra phonemes in other not in primary; skip.
                        }
                        DiffOp::Replace {
                            old_index: oi,
                            old_len: ol,
                            ..
                        } => {
                            for p in &old_slice[oi..oi + ol] {
                                result.push(Phoneme {
                                    value: p.clone(),
                                    diff: DiffKind::Fatal,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    result
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
    let mut json_pages: Option<std::path::PathBuf> = None;
    let mut file_paths: Vec<std::path::PathBuf> = vec![];
    {
        let mut iter = raw_args.into_iter();
        while let Some(arg) = iter.next() {
            if arg == "--json" {
                json_path = Some(
                    iter.next()
                        .ok_or_else(|| anyhow::anyhow!("--json requires a path"))?,
                );
            } else if arg == "--json-pages" {
                json_pages = Some(
                    iter.next()
                        .ok_or_else(|| anyhow::anyhow!("--json-pages requires a directory"))?
                        .into(),
                );
            } else {
                file_paths.push(std::path::PathBuf::from(arg));
            }
        }
    }
    anyhow::ensure!(
        json_path.is_none() || json_pages.is_none(),
        "Use either --json or --json-pages"
    );
    if let Some(directory) = &json_pages {
        std::fs::create_dir_all(directory.join("result-pages"))?;
        json_path = Some(
            directory
                .join("results.json")
                .to_string_lossy()
                .into_owned(),
        );
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
        jpreprocess::SystemDictionaryConfig::File(env!("JPREPROCESS_DICT_DIR").into()).load()?,
        None,
    );

    for (file_index, file) in file_paths.iter().enumerate() {
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
        let mut chunks = vec![];

        for (sentence_i, sentence) in sentences.iter().enumerate() {
            if json_path.is_none() {
                entries.clear();
            }
            if let Some(directory) = &json_pages {
                if entries.len() == 500 {
                    chunks.push(write_entry_chunk(
                        directory,
                        file_index,
                        chunks.len(),
                        &mut entries,
                    )?);
                }
            }
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
                let mut njd = jp.text_to_njd(sentence)?;
                njd.preprocess();
                let phrases = phrases_from_nodes(njd.nodes.iter().map(|node| {
                    (
                        node.get_string().to_owned(),
                        node.get_pron().to_string(),
                        node.get_chain_flag() == Some(true),
                    )
                }));
                Ok::<_, anyhow::Error>(Analysis {
                    labels: jpreprocess_jpcommon::njdnodes_to_features(&njd.nodes),
                    phrases,
                })
            }))
            .map_err(|e| anyhow::anyhow!("panicked! {:?}", e.downcast_ref::<String>()))
            .and_then(|r| r);
            jpreprocess_extraction_duration_ms +=
                jpreprocess_extraction_started.elapsed().as_secs_f64() * 1000.0;

            let (mut ojt_labels, mut jp_labels) = match (ojt_labels, jp_labels) {
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
                            jpreprocess::SystemDictionaryConfig::File(
                                env!("JPREPROCESS_DICT_DIR").into(),
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
            mark_phrase_diffs(&mut ojt_labels.phrases, &mut jp_labels.phrases);
            let ojt_phonemes = ojt_labels
                .labels
                .iter()
                .filter_map(|l| l.phoneme.c.clone())
                .collect::<Vec<_>>();
            let jp_phonemes = jp_labels
                .labels
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
                    openjtalk_phrases: ojt_labels.phrases,
                    jpreprocess_phrases: jp_labels.phrases,
                }));
            } else {
                let phonemes_ojt = phonemes_with_diff(&ojt_phonemes, &jp_phonemes);
                let phonemes_jp = phonemes_with_diff(&jp_phonemes, &ojt_phonemes);

                let is_fatal = phonemes_ojt
                    .iter()
                    .chain(phonemes_jp.iter())
                    .any(|p| matches!(p.diff, DiffKind::Fatal));
                let length_mismatch = ojt_phonemes.len() != jp_phonemes.len();

                if is_fatal {
                    if length_mismatch {
                        println!(
                            "{}\x1b[31mFatal mismatch: (length mismatch: OpenJTalk: {}, JPreprocess: {})\x1b[0m",
                            prefix,
                            ojt_phonemes.len(),
                            jp_phonemes.len()
                        );
                    } else {
                        println!("{}\x1b[31mFatal mismatch:\x1b[0m", prefix);
                    }
                    fatal_mismatches += 1;
                } else {
                    println!("{}\x1b[33mLight mismatch:\x1b[0m", prefix);
                    light_mismatches += 1;
                }

                let format_phonemes = |phonemes: &[Phoneme]| -> String {
                    phonemes
                        .iter()
                        .map(|p| match p.diff {
                            DiffKind::None => p.value.clone(),
                            DiffKind::Light => format!("\x1b[33m{}\x1b[0m", p.value),
                            DiffKind::Fatal => format!("\x1b[31m{}\x1b[0m", p.value),
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                println!("     Original: {}", sentence);
                println!("    OpenJTalk: {}", format_phonemes(&phonemes_ojt));
                println!("  JPreprocess: {}", format_phonemes(&phonemes_jp));

                let entry = MismatchEntry {
                    index: sentence_i,
                    original: sentence.to_string(),
                    openjtalk: phonemes_ojt,
                    jpreprocess: phonemes_jp,
                    openjtalk_phrases: ojt_labels.phrases,
                    jpreprocess_phrases: jp_labels.phrases,
                    length_mismatch: if length_mismatch { Some(true) } else { None },
                };
                entries.push(if is_fatal {
                    Entry::Fatal(entry)
                } else {
                    Entry::Light(entry)
                });
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

        if let Some(directory) = &json_pages {
            if !entries.is_empty() {
                chunks.push(write_entry_chunk(
                    directory,
                    file_index,
                    chunks.len(),
                    &mut entries,
                )?);
            }
        }
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
            chunks,
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

fn extract_fullcontext(text: &str) -> anyhow::Result<Analysis> {
    use open_jtalk_sys::*;
    let mut resources = OJT_RESOURCES.lock().unwrap();
    let Resources {
        mecab,
        njd,
        jpcommon,
    } = &mut *resources;
    mecab.refresh();
    let mecab_text = open_jtalk::text2mecab(text)?;
    anyhow::ensure!(mecab.analysis(mecab_text), "mecab.analysis() failed");
    let feature = mecab
        .get_feature()
        .ok_or_else(|| anyhow::anyhow!("mecab.get_feature()"))?;
    // NJD nodes are not exposed by open_jtalk; inspect the owned C resources under the mutex.
    unsafe {
        JPCommon_refresh(jpcommon);
        NJD_refresh(njd);
        mecab2njd(
            njd,
            feature as *const open_jtalk::MecabFeature as *mut *mut std::os::raw::c_char,
            mecab.get_size(),
        );
        njd_set_pronunciation(njd);
        njd_set_digit(njd);
        njd_set_accent_phrase(njd);
        njd_set_accent_type(njd);
        njd_set_unvoiced_vowel(njd);
        njd_set_long_vowel(njd);
        let mut nodes = Vec::new();
        let mut node = njd.head;
        while !node.is_null() {
            nodes.push((
                CStr::from_ptr(NJDNode_get_string(node))
                    .to_str()?
                    .to_owned(),
                CStr::from_ptr(NJDNode_get_pron(node)).to_str()?.to_owned(),
                NJDNode_get_chain_flag(node) == 1,
            ));
            node = (*node).next;
        }
        njd2jpcommon(jpcommon, njd);
        JPCommon_make_label(jpcommon);
        let features = JPCommon_get_label_feature(jpcommon);
        anyhow::ensure!(!features.is_null(), "JPCommon_get_label_feature()");
        let labels = (0..JPCommon_get_label_size(jpcommon))
            .map(|index| {
                Ok(jlabel::Label::from_str(
                    CStr::from_ptr(*features.add(index as usize)).to_str()?,
                )?)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Analysis {
            labels,
            phrases: phrases_from_nodes(nodes),
        })
    }
}

struct Resources {
    mecab: ManagedResource<Mecab>,
    njd: open_jtalk_sys::NJD,
    jpcommon: open_jtalk_sys::JPCommon,
}

impl Resources {
    fn new() -> Self {
        // Both C initializers fully initialize their resources before use.
        unsafe {
            let mut njd = MaybeUninit::uninit();
            let mut jpcommon = MaybeUninit::uninit();
            open_jtalk_sys::NJD_initialize(njd.as_mut_ptr());
            open_jtalk_sys::JPCommon_initialize(jpcommon.as_mut_ptr());
            Self {
                mecab: ManagedResource::initialize(),
                njd: njd.assume_init(),
                jpcommon: jpcommon.assume_init(),
            }
        }
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        unsafe {
            open_jtalk_sys::JPCommon_clear(&mut self.jpcommon);
            open_jtalk_sys::NJD_clear(&mut self.njd);
        }
    }
}

// The C resources are owned exclusively and accessed only while holding OJT_RESOURCES's mutex.
unsafe impl Send for Resources {}
