use open_jtalk::{text2mecab, JpCommon, ManagedResource, Mecab, Njd};
use std::str::FromStr;

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

fn main() -> anyhow::Result<()> {
    {
        let mut resources = OJT_RESOURCES.lock().unwrap();
        resources.mecab.load(DICT_DIR)?;
    }

    let mut total_matches = 0;
    let mut total_light_mismatches = 0;
    let mut total_fatal_mismatches = 0;
    let mut total_errors = 0;

    let jp = jpreprocess::JPreprocess::with_dictionaries(
        jpreprocess::SystemDictionaryConfig::Bundled(
            jpreprocess::kind::JPreprocessDictionaryKind::NaistJdic,
        )
        .load()?,
        None,
    );

    let files = std::env::args().skip(1).map(std::path::PathBuf::from);
    for file in files {
        let text = std::fs::read_to_string(&file)?;
        let sentences = lazy_regex::regex!("[。「」]")
            .split(&text)
            .map(|s| lazy_regex::regex_replace_all!(r"\s+", s, ""))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let sentences_size = sentences.len();
        let mut matches = 0;
        let mut light_mismatches = 0;
        let mut fatal_mismatches = 0;
        let mut errors = 0;
        for (sentence_i, sentence) in sentences.iter().enumerate() {
            let prefix = format!(
                "[{} : {} / {}]: ",
                file.file_name().unwrap().to_string_lossy(),
                sentence_i + 1,
                sentences_size
            );
            let ojt_labels = extract_fullcontext(sentence);
            let jp_labels = jp.extract_fullcontext(sentence);
            let (ojt_labels, jp_labels) = match (ojt_labels, jp_labels) {
                (Ok(ojt_labels), Ok(jp_labels)) => (ojt_labels, jp_labels),
                (r1, r2) => {
                    println!("{} \x1b[31mError:\x1b[0m", prefix);
                    println!("     Original: {}", sentence);
                    println!("    OpenJTalk: {:?}", r1.map(|_| ()));
                    println!("  JPreprocess: {:?}", r2.map(|_| ()));
                    errors += 1;
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

                    if differences.iter().any(|d| d == &Some(Difference::Fatal)) {
                        println!("{}\x1b[31mFatal mismatch:\x1b[0m", prefix,);
                    } else {
                        println!("{}\x1b[33mLight mismatch:\x1b[0m", prefix,);
                    }
                    println!("     Original: {}", sentence);
                    light_mismatches += 1;
                    println!("    OpenJTalk: {}", ojt_buffer.trim());
                    println!("  JPreprocess: {}", jp_buffer.trim());
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
                }
            }
        }

        println!(
            "{}: {} matches, {} light mismatches, {} fatal mismatches, {} errors",
            file.file_name().unwrap().to_string_lossy(),
            matches,
            light_mismatches,
            fatal_mismatches,
            errors
        );

        total_matches += matches;
        total_light_mismatches += light_mismatches;
        total_fatal_mismatches += fatal_mismatches;
        total_errors += errors;
    }

    println!(
        "Total: {} matches, {} light mismatches, {} fatal mismatches, {} errors",
        total_matches, total_light_mismatches, total_fatal_mismatches, total_errors
    );

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
