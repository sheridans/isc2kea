use crate::{convert_config, MigrationOptions};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Cursor, Read, Write};
use xmltree::{Element, XMLNode};

use super::VerifyArgs;

pub(crate) fn run_verify(args: VerifyArgs) -> Result<()> {
    let mut file = File::open(&args.r#in)
        .with_context(|| format!("Failed to open input file: {}", args.r#in.display()))?;
    let mut input_buf = Vec::new();
    file.read_to_end(&mut input_buf)
        .with_context(|| format!("Failed to read input file: {}", args.r#in.display()))?;

    let options = MigrationOptions {
        fail_if_existing: args.fail_if_existing,
        verbose: args.verbose,
        backend: args.backend.clone(),
        create_subnets: args.create_subnets,
        force_subnets: args.force_subnets,
        create_options: args.create_options,
        force_options: args.force_options,
        enable_backend: args.enable_backend,
    };

    let mut output_buf = Vec::new();
    let _stats = convert_config(Cursor::new(&input_buf), &mut output_buf, &options)?;

    let input_str = normalize_xml(&input_buf)
        .with_context(|| format!("Failed to normalize input: {}", args.r#in.display()))?;
    let output_str = normalize_xml(&output_buf).context("Failed to normalize converted output")?;

    if input_str == output_str {
        if !args.quiet {
            println!("No changes.");
        }
        return Ok(());
    }

    if !args.quiet {
        let diff = similar::TextDiff::from_lines(&input_str, &output_str);
        let mut out = io::stdout().lock();
        let unified = diff
            .unified_diff()
            .context_radius(3)
            .header("original", "converted")
            .to_string();
        write!(out, "{}", unified)?;
    }

    Err(anyhow::anyhow!("verify: changes detected"))
}

fn normalize_xml(input: &[u8]) -> Result<String> {
    let root = Element::parse(Cursor::new(input)).context("Failed to parse XML")?;
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write_element(&root, 0, &mut out);
    Ok(out)
}

fn write_element(el: &Element, indent: usize, out: &mut String) {
    let indent_str = " ".repeat(indent);
    out.push_str(&indent_str);
    out.push('<');
    out.push_str(&el.name);

    let mut attrs: Vec<_> = el.attributes.iter().collect();
    attrs.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(&escape_xml(v));
        out.push('"');
    }

    if el.children.is_empty() {
        out.push_str(" />\n");
        return;
    }

    if el.children.len() == 1 {
        if let XMLNode::Text(text) = &el.children[0] {
            let trimmed = text.trim();
            out.push('>');
            out.push_str(&escape_xml(trimmed));
            out.push_str("</");
            out.push_str(&el.name);
            out.push_str(">\n");
            return;
        }
    }

    out.push_str(">\n");
    for child in &el.children {
        match child {
            XMLNode::Element(child_el) => write_element(child_el, indent + 2, out),
            XMLNode::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                out.push_str(&" ".repeat(indent + 2));
                out.push_str(&escape_xml(trimmed));
                out.push('\n');
            }
            XMLNode::CData(data) => {
                out.push_str(&" ".repeat(indent + 2));
                out.push_str("<![CDATA[");
                out.push_str(data);
                out.push_str("]]>\n");
            }
            XMLNode::Comment(comment) => {
                out.push_str(&" ".repeat(indent + 2));
                out.push_str("<!--");
                out.push_str(comment);
                out.push_str("-->\n");
            }
            XMLNode::ProcessingInstruction(pi, data) => {
                out.push_str(&" ".repeat(indent + 2));
                out.push_str("<?");
                out.push_str(pi);
                if let Some(data) = data {
                    out.push(' ');
                    out.push_str(data);
                }
                out.push_str("?>\n");
            }
        }
    }

    out.push_str(&indent_str);
    out.push_str("</");
    out.push_str(&el.name);
    out.push_str(">\n");
}

fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}
