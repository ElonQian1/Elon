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

/// Returns the textual function region beginning at the exact anchor symbol and ending before
/// the next peer function declaration. Ledger symbols deliberately include enough signature text
/// to avoid prefix matches such as `fn map` versus `fn map_connection`.
pub(super) fn symbol_span<'source>(source: &'source str, symbol: &str) -> Option<&'source str> {
    let start = source.find(symbol)?;
    let after_symbol = start.checked_add(symbol.len())?;
    let tail = source.get(after_symbol..)?;
    let end = NEXT_FUNCTION_PREFIXES
        .iter()
        .filter_map(|prefix| tail.find(prefix))
        .min()
        .map_or(source.len(), |offset| after_symbol + offset);
    source.get(start..end)
}
