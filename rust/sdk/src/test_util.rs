/// parses hex from cbor.me format
pub fn from_cbor_me(text: &str) -> anyhow::Result<Vec<u8>> {
    let parts = text
        .split('\n')
        .filter_map(|x| x.split('#').next())
        .flat_map(|x| x.split_whitespace())
        .collect::<String>();
    Ok(hex::decode(parts)?)
}
