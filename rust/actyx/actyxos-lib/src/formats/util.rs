/// Returns a cross-platform-filename-safe version of any string. All potentially
/// troublesome chars are replaced with an underscore ('_').
///
/// Do not apply this function to full paths, as it will sanitize '/' and '\';
/// it should only be used on directory or file names (i.e. path segments).
///
/// (Slightly adapted from https://docs.rs/app_dirs/1.2.1/src/app_dirs/utils.rs.html)
#[allow(dead_code)]
pub fn sanitized(component: &str) -> String {
    let sanitize_with = '_'.to_string();
    let mut buf = String::with_capacity(component.len());
    for (i, c) in component.chars().enumerate() {
        let is_valid = ('a'..='z').contains(&c) // lower letter
            || ('A'..='Z').contains(&c) // upper letter
            || ('0'..='9').contains(&c) // number
            || (c == '-') // hyphen
            || (c == '_') // underscore
            || (c == '.' && i != 0); // period (disallow accidentally hidden folders)
        if is_valid {
            buf.push(c);
        } else {
            buf.push_str(&sanitize_with);
        }
    }
    buf
}
