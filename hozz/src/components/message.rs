use dioxus::prelude::*;

const MARKDOWN_STYLE: &str = r#"
.markdown-body {
    color: rgb(244 244 245);
    line-height: 1.65;
    overflow-wrap: anywhere;
}

.markdown-body > * + * {
    margin-top: 0.9rem;
}

.markdown-body p {
    margin: 0;
}

.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {
    margin: 1.2rem 0 0.55rem;
    font-weight: 600;
    line-height: 1.3;
    color: rgb(250 250 250);
}

.markdown-body h1 { font-size: 1.4rem; }
.markdown-body h2 { font-size: 1.2rem; }
.markdown-body h3 { font-size: 1.05rem; }

.markdown-body ul,
.markdown-body ol {
    margin: 0.9rem 0;
    padding-left: 0;
}

.markdown-body ul,
.markdown-body ol {
    list-style: none;
}

.markdown-body ol {
    counter-reset: item;
}

.markdown-body li {
    position: relative;
    display: block;
    min-height: 1.6rem;
    margin: 0.35rem 0;
    padding-left: 2rem;
}

.markdown-body li > ul,
.markdown-body li > ol {
    margin-top: 0.35rem;
    margin-bottom: 0.35rem;
}

.markdown-body ul > li::before {
    content: "";
    position: absolute;
    left: 0.18rem;
    top: 0.52rem;
    width: 0.7rem;
    height: 0.7rem;
    border-radius: 999px;
    background: linear-gradient(135deg, rgb(34 211 238), rgb(59 130 246));
    box-shadow: 0 0 0 3px rgba(34, 211, 238, 0.12);
}

.markdown-body ul > li:not(:last-child)::after {
    content: "";
    position: absolute;
    left: 0.52rem;
    top: 1.25rem;
    bottom: -0.7rem;
    border-left: 1px dashed rgba(34, 211, 238, 0.35);
}

.markdown-body ol > li {
    counter-increment: item;
}

.markdown-body ol > li::before {
    content: counter(item);
    position: absolute;
    left: 0;
    top: 0.1rem;
    width: 1.25rem;
    height: 1.25rem;
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.72rem;
    font-weight: 700;
    color: rgb(253 224 71);
    border: 1px solid rgba(250, 204, 21, 0.45);
    background: rgba(250, 204, 21, 0.08);
}

.markdown-body ol > li:not(:last-child)::after {
    content: "";
    position: absolute;
    left: 0.62rem;
    top: 1.5rem;
    bottom: -0.7rem;
    border-left: 1px dashed rgba(250, 204, 21, 0.32);
}

.markdown-body li:has(> input[type="checkbox"]) {
    padding-left: 0;
}

.markdown-body li:has(> input[type="checkbox"])::before,
.markdown-body li:has(> input[type="checkbox"])::after {
    display: none;
}

.markdown-body li input[type="checkbox"] {
    margin-right: 0.65rem;
    accent-color: rgb(34 211 238);
    transform: translateY(1px);
}

.markdown-body blockquote {
    margin: 1rem 0;
    padding-left: 0.9rem;
    border-left: 3px solid rgb(63 63 70);
    color: rgb(161 161 170);
}

.markdown-body pre {
    margin: 0;
    padding: 0.9rem 1rem;
    overflow-x: auto;
    border-radius: 0 0 0.9rem 0.9rem;
    border: 0;
}

.markdown-body code {
    font-size: 0.92em;
    color: rgb(34 211 238);
}

.markdown-body :not(pre) > code {
    padding: 0.12rem 0.35rem;
    border-radius: 0.35rem;
    background: rgba(255, 255, 255, 0.06);
}

.markdown-body a {
    color: rgb(103 232 249);
    text-decoration: underline;
    text-underline-offset: 0.18rem;
}

.markdown-body hr {
    margin: 1rem 0;
    border: 0;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.markdown-body table {
    width: 100%;
    border-collapse: collapse;
    min-width: 100%;
    margin: 0;
}

.markdown-body .md-table-wrap {
    margin: 1rem 0;
    overflow-x: auto;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 0.9rem;
    background: rgba(255, 255, 255, 0.02);
}

.markdown-body th,
.markdown-body td {
    padding: 0.55rem 0.7rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    text-align: left;
}

.markdown-body thead th {
    background: rgba(255, 255, 255, 0.04);
    color: rgb(250 250 250);
    font-weight: 600;
}

.markdown-body .md-code-block {
    margin: 1rem 0;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 0.9rem;
    overflow: hidden;
    background: rgb(9 9 11);
}

.markdown-body .md-code-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.55rem 0.8rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
}

.markdown-body .md-code-lang {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgb(161 161 170);
}

.markdown-body .md-code-copy {
    border: 0;
    border-radius: 0.5rem;
    padding: 0.22rem 0.55rem;
    background: rgba(255, 255, 255, 0.06);
    color: rgb(228 228 231);
    font-size: 0.74rem;
    cursor: pointer;
    transition: background-color 120ms ease;
}

.markdown-body .md-code-copy:hover {
    background: rgba(255, 255, 255, 0.12);
}
"#;

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn extract_code_language(block: &str) -> String {
    let marker = "class=\"language-";
    if let Some(start) = block.find(marker) {
        let rest = &block[start + marker.len()..];
        let end = rest.find(['"', ' ', ',']).unwrap_or(rest.len());
        let lang = rest[..end].trim();
        if !lang.is_empty() {
            return lang.to_string();
        }
    }

    "text".to_string()
}

fn wrap_code_blocks(html: &str) -> String {
    let mut output = String::with_capacity(html.len() + 512);
    let mut rest = html;

    while let Some(pre_start) = rest.find("<pre") {
        output.push_str(&rest[..pre_start]);
        rest = &rest[pre_start..];

        let Some(pre_end) = rest.find("</pre>") else {
            output.push_str(rest);
            return output;
        };

        let block = &rest[..pre_end + "</pre>".len()];
        let language = extract_code_language(block);
        let safe_language = escape_html(&language);
        output.push_str(&format!(
            "<div class=\"md-code-block\"><div class=\"md-code-header\"><span class=\"md-code-lang\">{}</span><button class=\"md-code-copy\" onclick=\"(async(btn)=>{{const code=btn.closest('.md-code-block')?.querySelector('code');const text=code?.innerText??'';try{{await navigator.clipboard.writeText(text);const prev=btn.innerText;btn.innerText='Copied';setTimeout(()=>btn.innerText=prev,1200);}}catch(_e){{}}}})(this)\">Copy</button></div>{}</div>",
            safe_language,
            block,
        ));

        rest = &rest[pre_end + "</pre>".len()..];
    }

    output.push_str(rest);
    output
}

fn enhance_markdown_html(html: String) -> String {
    let html = wrap_code_blocks(&html);
    html.replace("<table>", "<div class=\"md-table-wrap\"><table>")
        .replace("</table>", "</table></div>")
}

#[component]
pub fn MarkdownMessage(content: String) -> Element {
    let html = use_memo(move || {
        let mut options = comrak::Options::default();
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.tasklist = true;
        options.extension.autolink = true;
        options.extension.tagfilter = true;
        options.parse.tasklist_in_table = true;
        options.render.r#unsafe = true;

        let adapter =
            comrak::plugins::syntect::SyntectAdapter::new(Some("base16-ocean.dark"));
        let mut plugins = comrak::options::Plugins::default();
        plugins.render.codefence_syntax_highlighter = Some(&adapter);

        let rendered =
            comrak::markdown_to_html_with_plugins(&content, &options, &plugins);
        enhance_markdown_html(rendered)
    });

    rsx! {
        style { "{MARKDOWN_STYLE}" }
        div {
            class: "markdown-body prose prose-invert prose-sm max-w-none",
            dangerous_inner_html: "{html}"
        }
    }
}
