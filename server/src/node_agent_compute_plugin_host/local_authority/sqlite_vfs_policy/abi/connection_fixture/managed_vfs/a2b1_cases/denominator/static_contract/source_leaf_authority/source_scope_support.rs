const NEXT_FUNCTION_PREFIXES: &[&str] = &[
    "\nfn ",
    "\nunsafe fn ",
    "\nunsafe extern ",
    "\npub fn ",
    "\npub(super) fn ",
    "\npub(crate) fn ",
    "\npub unsafe fn ",
    "\npub(super) unsafe fn ",
    "\npub(crate) unsafe fn ",
    "\npub unsafe extern ",
    "\npub(super) unsafe extern ",
    "\npub(crate) unsafe extern ",
    "\n    fn ",
    "\n    unsafe fn ",
    "\n    unsafe extern ",
    "\n    pub fn ",
    "\n    pub(super) fn ",
    "\n    pub(crate) fn ",
    "\n    pub unsafe fn ",
    "\n    pub(super) unsafe fn ",
    "\n    pub(crate) unsafe fn ",
    "\n    pub unsafe extern ",
    "\n    pub(super) unsafe extern ",
    "\n    pub(crate) unsafe extern ",
    "\n    pub(in ",
];

pub(super) fn symbol_span<'source>(source: &'source str, symbol: &str) -> Option<&'source str> {
    let mut matches = source.match_indices(symbol);
    let (start, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let after_symbol = start.checked_add(symbol.len())?;
    let tail = source.get(after_symbol..)?;
    let end = NEXT_FUNCTION_PREFIXES
        .iter()
        .filter_map(|prefix| tail.find(prefix))
        .min()
        .map_or(source.len(), |offset| after_symbol + offset);
    source.get(start..end)
}

pub(super) fn is_lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn lower_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}
