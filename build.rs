const DIC_DIR_NAME: &str = "open_jtalk_dic_utf_8-1.11";
fn download_open_jtalk_dict(dist: &std::path::Path) -> anyhow::Result<()> {
    let download_url = format!(
        "https://github.com/r9y9/open_jtalk/releases/download/v1.11.1/{DIC_DIR_NAME}.tar.gz"
    );

    let res = ureq::get(&download_url).call()?;
    anyhow::ensure!(res.status() == 200, "{}", res.status());

    let bytes = res.into_reader();
    let dict_tar = flate2::read::GzDecoder::new(bytes);

    let mut dict_archive = tar::Archive::new(dict_tar);
    dict_archive.unpack(dist)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let dist = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    if !dist.exists() {
        download_open_jtalk_dict(&dist)?;
    }
    Ok(())
}
