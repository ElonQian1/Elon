//! 嵌入式 Admin 控制台静态 HTML/CSS/JS 资源（从 admin.rs 抽出）。
//!
//! HTML/CSS/JS 内容存储在 `assets/admin.html`，编译期通过 `include_str!` 内联。
//! 修改界面时直接编辑 `assets/admin.html`，无需触碰 Rust 代码。

pub(crate) const ADMIN_HTML: &str = include_str!("assets/admin.html");
