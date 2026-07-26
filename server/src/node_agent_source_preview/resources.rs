use anyhow::{Context, Result};
use quick_xml::{escape::unescape, events::Event, Reader};
use std::{collections::HashMap, fs, path::Path};

#[derive(Default)]
pub(crate) struct AndroidResources {
    values: HashMap<String, String>,
    drawables: HashMap<String, (String, f32)>,
}

impl AndroidResources {
    pub(crate) fn load(layout_path: &Path) -> Result<Self> {
        let res_dir = layout_path
            .parent()
            .and_then(Path::parent)
            .context("布局不在 res/layout 下")?;
        let mut this = Self::default();
        for entry in fs::read_dir(res_dir)
            .with_context(|| format!("读取资源目录失败: {}", res_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type()?.is_dir()
                || !entry.file_name().to_string_lossy().starts_with("values")
            {
                continue;
            }
            for file in fs::read_dir(path)? {
                let file = file?.path();
                if file.extension().and_then(|v| v.to_str()) == Some("xml") {
                    this.read_values_file(&file)?;
                }
            }
        }
        for entry in fs::read_dir(res_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir()
                || !entry.file_name().to_string_lossy().starts_with("drawable")
            {
                continue;
            }
            for file in fs::read_dir(entry.path())? {
                let path = file?.path();
                if path.extension().and_then(|v| v.to_str()) == Some("xml") {
                    this.read_drawable_file(&path)?;
                }
            }
        }
        Ok(this)
    }

    fn read_values_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        self.read_values_xml(&content)
    }

    fn read_values_xml(&mut self, content: &str) -> Result<()> {
        let mut reader = Reader::from_str(&content);
        let mut buf = Vec::new();
        let mut active: Option<(String, String, String)> = None;
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(event) => {
                    let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                    let name = event.attributes().flatten().find_map(|attribute| {
                        (attribute.key.as_ref() == b"name")
                            .then(|| String::from_utf8_lossy(&attribute.value).to_string())
                    });
                    if let Some(name) = name {
                        active = Some((tag, name, String::new()));
                    }
                }
                Event::Text(text) => {
                    if let Some((_, _, value)) = active.as_mut() {
                        value.push_str(text.decode()?.as_ref());
                    }
                }
                Event::CData(text) => {
                    if let Some((_, _, value)) = active.as_mut() {
                        value.push_str(text.decode()?.as_ref());
                    }
                }
                Event::GeneralRef(reference) => {
                    if let Some((_, _, value)) = active.as_mut() {
                        let entity = format!("&{};", reference.decode()?);
                        value.push_str(unescape(&entity)?.as_ref());
                    }
                }
                Event::End(event) => {
                    let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                    if active.as_ref().is_some_and(|(kind, _, _)| kind == &tag) {
                        let (kind, name, value) = active.take().expect("active value exists");
                        self.values
                            .insert(format!("@{kind}/{name}"), value.trim().to_string());
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }

    pub(crate) fn resolve(&self, raw: &str) -> String {
        self.values
            .get(raw)
            .cloned()
            .unwrap_or_else(|| raw.to_string())
    }

    pub(crate) fn drawable(&self, raw: &str) -> Option<(String, f32)> {
        self.drawables.get(raw).cloned()
    }

    fn read_drawable_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let mut reader = Reader::from_str(&content);
        let mut buf = Vec::new();
        let mut color = None;
        let mut radius = 0.0;
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(event) | Event::Empty(event) => {
                    let tag = String::from_utf8_lossy(event.name().as_ref()).to_string();
                    for attribute in event.attributes().flatten() {
                        let key = String::from_utf8_lossy(attribute.key.as_ref());
                        let value = String::from_utf8_lossy(&attribute.value).to_string();
                        if tag == "solid" && key.ends_with("color") {
                            color = Some(self.resolve(&value));
                        }
                        if tag == "corners" && key.ends_with("radius") {
                            radius = scalar(&self.resolve(&value));
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        if color.is_some() || radius > 0.0 {
            let name = path
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or_default();
            self.drawables.insert(
                format!("@drawable/{name}"),
                (color.unwrap_or_else(|| "transparent".into()), radius),
            );
        }
        Ok(())
    }
}

pub(crate) fn scalar(raw: &str) -> f32 {
    raw.trim()
        .trim_end_matches("dp")
        .trim_end_matches("sp")
        .trim_end_matches("px")
        .parse()
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_escaped_android_string_values() {
        let mut resources = AndroidResources::default();
        resources
            .read_values_xml(r#"<resources><string name="label">A &amp; B</string></resources>"#)
            .unwrap();

        assert_eq!(resources.resolve("@string/label"), "A & B");
    }
}
