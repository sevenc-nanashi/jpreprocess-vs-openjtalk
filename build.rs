use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    let dist = PathBuf::from(std::env::var("OUT_DIR")?);
    let source = dist.join("naist-jdic-0.1.3");
    let res =
        ureq::get("https://github.com/jpreprocess/naist-jdic/archive/refs/tags/v0.1.3.tar.gz")
            .call()?;
    let dict_tar = flate2::read::GzDecoder::new(res.into_body().into_reader());
    tar::Archive::new(dict_tar).unpack(&dist)?;

    let jp_dict = dist.join("jpreprocess-dict");
    jpreprocess_dictionary::dictionary::to_dict::JPreprocessDictionaryBuilder::default()
        .build_dictionary(&source, &jp_dict)?;

    let ojt_dict = dist.join("openjtalk-dict");
    std::fs::create_dir_all(&ojt_dict)?;
    open_jtalk::mecab_dict_index(&[
        "mecab-dict-index",
        "-d",
        source.to_str().unwrap(),
        "-o",
        ojt_dict.to_str().unwrap(),
        "-f",
        "UTF-8",
        "-t",
        "UTF-8",
    ]);
    // mecab_dict_index does not return its exit status; verify the generated dictionary loads.
    let mut mecab = open_jtalk::ManagedResource::<open_jtalk::Mecab>::initialize();
    mecab.load(ojt_dict.to_str().unwrap())?;

    if std::env::var("CARGO_CFG_TARGET_OS")? == "emscripten" {
        for (directory, mount) in [
            (&jp_dict, "/jpreprocess-dict"),
            (&ojt_dict, "/openjtalk-dict"),
        ] {
            println!("cargo:rustc-link-arg=--preload-file");
            println!("cargo:rustc-link-arg={}@{mount}", directory.display());
        }
        println!("cargo:rustc-env=JPREPROCESS_DICT_DIR=/jpreprocess-dict");
        println!("cargo:rustc-env=OPENJTALK_DICT_DIR=/openjtalk-dict");
    } else {
        println!("cargo:rustc-env=JPREPROCESS_DICT_DIR={}", jp_dict.display());
        println!("cargo:rustc-env=OPENJTALK_DICT_DIR={}", ojt_dict.display());
    }
    Ok(())
}
