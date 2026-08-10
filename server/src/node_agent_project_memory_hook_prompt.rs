pub(super) fn bounded_paths<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    max_paths: usize,
    max_chars: usize,
) -> Vec<&'a str> {
    let mut selected = Vec::new();
    let mut used = 0usize;
    for path in paths {
        let separator = usize::from(!selected.is_empty()) * 2;
        let next = used
            .saturating_add(separator)
            .saturating_add(path.chars().count());
        if next > max_chars {
            continue;
        }
        selected.push(path);
        used = next;
        if selected.len() >= max_paths {
            break;
        }
    }
    selected
}
