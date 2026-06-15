use super::symbol_index_compression_types::{
    CompressedContextBlock, CompressionLevel, SymbolCompressedContext,
};

pub(crate) fn render_compressed_context(context: &SymbolCompressedContext) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<compressed_context budgetTokens=\"{}\" usedTokens=\"{}\" originalTokens=\"{}\" savedTokens=\"{}\" dropped=\"{}\">\n",
        context.budget_tokens,
        context.used_tokens,
        context.original_tokens,
        context.saved_tokens,
        context.dropped_count
    ));
    for block in context
        .blocks
        .iter()
        .filter(|block| block.level != CompressionLevel::Drop)
        .take(20)
    {
        out.push_str(&format!(
            "## #{} {}:{} `{}`\n",
            block.rank,
            xml_escape(&block.file_path),
            rank_line_hint(block),
            xml_escape(&block.title)
        ));
        out.push_str(&format!(
            "Compression: {} | decision={} | source={} | sources={} | tokens {} -> {}\n",
            block.level.as_str(),
            block.decision.as_str(),
            xml_escape(&block.source),
            xml_escape(&block.sources.join("+")),
            block.original_tokens,
            block.compressed_tokens
        ));
        out.push_str(&format!(
            "Reason: {}\n",
            xml_escape(&block.reasons.join("; "))
        ));
        out.push_str("```text\n");
        out.push_str(&block.content);
        if !block.content.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n\n");
    }
    out.push_str("</compressed_context>\n");
    out
}

fn rank_line_hint(block: &CompressedContextBlock) -> usize {
    block
        .content
        .lines()
        .find_map(|line| line.strip_prefix("lines: "))
        .and_then(|line| line.split('-').next())
        .and_then(|line| line.parse().ok())
        .unwrap_or_default()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
