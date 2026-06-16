#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedLiteral {
    pub text: String,
    pub reason: &'static str,
}

const PROTECTED_TERMS: &[&str] = &[
    "ZeroClaw",
    "ZeroClaw Maturity Framework",
    "zerocode",
    "ACP",
    "MCP",
    "LINE",
    "Matrix",
    "Discord",
    "Telegram",
    "Slack",
    "Lark",
    "Mattermost",
    "Nextcloud Talk",
    "WhatsApp",
    "OpenAI",
    "Anthropic",
    "Ollama",
    "OpenRouter",
    "Bedrock",
    "Gemini",
    "Azure",
    "Groq",
    "Mistral",
    "xAI",
    "TOML",
    "YAML",
    "JSON",
    "HTTP",
    "HTTPS",
    "WSS",
    "OAuth",
];

const COMMAND_PREFIXES: &[&str] = &[
    "zeroclaw",
    "zerocode",
    "cargo",
    "git",
    "gh",
    "curl",
    "docker",
    "systemctl",
    "journalctl",
    "mdbook",
    "msgfmt",
    "msgmerge",
    "msgcat",
    "msgattrib",
    "msginit",
];

pub fn protected_literals(text: &str) -> Vec<ProtectedLiteral> {
    let mut literals = Vec::new();
    collect_protected_terms(text, &mut literals);
    collect_inline_code_literals(text, &mut literals);
    collect_fenced_code_literals(text, &mut literals);
    sort_dedup_literals(&mut literals);
    literals
}

pub fn missing_protected_literal(source: &str, translation: &str) -> Option<ProtectedLiteral> {
    if translation.trim().is_empty() {
        return None;
    }

    protected_literals(source)
        .into_iter()
        .find(|literal| !translation.contains(&literal.text))
}

pub fn missing_protected_literal_reason(source: &str, translation: &str) -> Option<&'static str> {
    missing_protected_literal(source, translation).map(|literal| literal.reason)
}

pub fn preservation_prompt(source: &str) -> Option<String> {
    let literals = protected_literals(source);
    if literals.is_empty() {
        return None;
    }

    let mut out = String::from("Preserve these exact substrings unchanged:");
    for literal in literals {
        out.push_str("\n- ");
        out.push_str(&literal.text);
    }
    Some(out)
}

fn collect_protected_terms(text: &str, literals: &mut Vec<ProtectedLiteral>) {
    for term in PROTECTED_TERMS {
        if text.contains(term) {
            push_literal(literals, term, "protected product/protocol name changed");
        }
    }
}

fn collect_inline_code_literals(text: &str, literals: &mut Vec<ProtectedLiteral>) {
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        rest = &rest[start + 1..];
        if rest.starts_with("``") {
            continue;
        }
        let Some(end) = rest.find('`') else {
            break;
        };
        let literal = &rest[..end];
        rest = &rest[end + 1..];
        collect_machine_literal(literal, literals);
    }
}

fn collect_fenced_code_literals(text: &str, literals: &mut Vec<ProtectedLiteral>) {
    let mut fence_language: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if fence_language.is_some() {
                fence_language = None;
            } else {
                fence_language = Some(
                    trimmed
                        .trim_start_matches('`')
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_ascii_lowercase(),
                );
            }
            continue;
        }

        let Some(language) = fence_language.as_deref() else {
            continue;
        };
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match language {
            "toml" => {
                collect_machine_literal(trimmed, literals);
                collect_toml_literals(trimmed, literals);
            }
            "yaml" | "yml" => {
                collect_machine_literal(trimmed, literals);
                collect_yaml_literals(trimmed, literals);
            }
            "json" => {
                collect_machine_literal(trimmed, literals);
                collect_json_literals(trimmed, literals);
            }
            _ => collect_generic_code_literal(trimmed, literals),
        }
    }
}

fn collect_machine_literal(text: &str, literals: &mut Vec<ProtectedLiteral>) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    if let Some(command) = command_literal(trimmed) {
        push_literal(literals, &command, "machine-facing code literal changed");
    }
    for flag in trimmed.split_whitespace().filter(|part| is_cli_flag(part)) {
        push_literal(literals, flag, "machine-facing code literal changed");
    }

    if is_toml_section(trimmed)
        || is_env_var(trimmed)
        || is_path_like(trimmed)
        || is_url_like(trimmed)
        || is_symbol_like(trimmed)
        || is_label_like(trimmed)
        || is_structured_inline_literal(trimmed)
    {
        push_literal(literals, trimmed, "machine-facing code literal changed");
    }
}

fn collect_generic_code_literal(text: &str, literals: &mut Vec<ProtectedLiteral>) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    if let Some(command) = command_literal(trimmed) {
        push_literal(literals, &command, "machine-facing code literal changed");
    }
    for flag in trimmed.split_whitespace().filter(|part| is_cli_flag(part)) {
        push_literal(literals, flag, "machine-facing code literal changed");
    }

    if is_env_var(trimmed)
        || is_path_like(trimmed)
        || is_url_like(trimmed)
        || is_symbol_like(trimmed)
        || is_label_like(trimmed)
    {
        push_literal(literals, trimmed, "machine-facing code literal changed");
    }
}

fn collect_toml_literals(line: &str, literals: &mut Vec<ProtectedLiteral>) {
    if is_toml_section(line) {
        push_literal(literals, line.trim(), "machine-facing code literal changed");
    } else if let Some((key, _)) = line.split_once('=') {
        let key = key.trim();
        if is_config_key(key) {
            push_literal(literals, key, "machine-facing code literal changed");
        }
    }
}

fn collect_yaml_literals(line: &str, literals: &mut Vec<ProtectedLiteral>) {
    let Some((key, _)) = line.split_once(':') else {
        return;
    };
    let key = key.trim().trim_matches('"').trim_matches('\'');
    if is_config_key(key) {
        push_literal(literals, key, "machine-facing code literal changed");
    }
}

fn collect_json_literals(line: &str, literals: &mut Vec<ProtectedLiteral>) {
    let trimmed = line
        .trim_start()
        .trim_start_matches('{')
        .trim_start_matches(',');
    let Some(rest) = trimmed.strip_prefix('"') else {
        return;
    };
    let Some((key, after)) = rest.split_once('"') else {
        return;
    };
    if after.trim_start().starts_with(':') && is_config_key(key) {
        push_literal(literals, key, "machine-facing code literal changed");
    }
}

fn command_literal(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let command = trimmed.split_whitespace().next()?;
    if !COMMAND_PREFIXES.contains(&command) {
        return None;
    }
    if trimmed.chars().all(is_command_literal_char) {
        return Some(trimmed.to_string());
    }

    let mut keep = vec![command];
    for part in trimmed.split_whitespace().skip(1) {
        if !part.chars().all(is_command_literal_char) {
            break;
        }
        keep.push(part);
    }
    Some(keep.join(" "))
}

fn is_command_literal_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c.is_ascii_whitespace()
        || matches!(
            c,
            '-' | '_' | ':' | '/' | '.' | '=' | '[' | ']' | '<' | '>' | '"' | '\'' | '@'
        )
}

fn is_cli_flag(text: &str) -> bool {
    let text = text.trim_end_matches([',', '.', ';', ':']);
    text.starts_with("--")
        && text.len() > 2
        && text[2..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_env_var(text: &str) -> bool {
    let text = text.trim();
    text.len() >= 3
        && text.contains('_')
        && text
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn is_url_like(text: &str) -> bool {
    text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("wss://")
        || text.starts_with("ws://")
}

fn is_path_like(text: &str) -> bool {
    text.starts_with('/')
        || text.starts_with("./")
        || text.starts_with("../")
        || text.contains("src/")
        || text.contains("docs/")
        || text.contains(".rs")
        || text.contains(".toml")
        || text.contains(".yaml")
        || text.contains(".yml")
        || text.contains(".json")
        || text.contains(".md")
}

fn is_symbol_like(text: &str) -> bool {
    let text = text.trim();
    text.ends_with("()")
        || text.contains("::")
        || text.contains("->")
        || text.contains("fl!(")
        || text.contains("env::var")
}

fn is_label_like(text: &str) -> bool {
    let text = text.trim();
    let Some((prefix, value)) = text.split_once(':') else {
        return false;
    };
    !prefix.is_empty()
        && !value.is_empty()
        && !text.contains(' ')
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '/'))
}

fn is_structured_inline_literal(text: &str) -> bool {
    let text = text.trim();
    if text.contains(' ') {
        return text.starts_with('{') || text.contains("\":");
    }
    text.contains('_')
        || text.contains('.')
        || text.contains('/')
        || text.contains('=')
        || (text.starts_with('"') && text.ends_with('"'))
        || (text.starts_with('\'') && text.ends_with('\''))
}

fn is_config_key(text: &str) -> bool {
    is_toml_key_path(text)
}

fn is_toml_section(text: &str) -> bool {
    let text = text.trim();
    let section = if text.starts_with("[[") && text.ends_with("]]") {
        &text[2..text.len() - 2]
    } else if text.starts_with('[') && text.ends_with(']') {
        &text[1..text.len() - 1]
    } else {
        return false;
    };
    is_toml_key_path(section.trim())
}

fn is_toml_key_path(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    let mut start = 0;
    let mut quote = None;
    for (idx, c) in text.char_indices() {
        if let Some(active_quote) = quote {
            if c == active_quote {
                quote = None;
            }
            continue;
        }

        match c {
            '"' | '\'' => quote = Some(c),
            '.' => {
                if !is_toml_key_segment(&text[start..idx]) {
                    return false;
                }
                start = idx + c.len_utf8();
            }
            _ => {}
        }
    }

    quote.is_none() && is_toml_key_segment(&text[start..])
}

fn is_toml_key_segment(text: &str) -> bool {
    let text = text.trim();
    is_bare_toml_key(text) || is_quoted_toml_key(text)
}

fn is_bare_toml_key(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_quoted_toml_key(text: &str) -> bool {
    text.len() >= 2
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('\'') && text.ends_with('\'')))
}

fn push_literal(literals: &mut Vec<ProtectedLiteral>, text: &str, reason: &'static str) {
    literals.push(ProtectedLiteral {
        text: text.to_string(),
        reason,
    });
}

fn sort_dedup_literals(literals: &mut Vec<ProtectedLiteral>) {
    literals.sort_by(|a, b| a.text.cmp(&b.text).then(a.reason.cmp(b.reason)));
    literals.dedup_by(|a, b| a.text == b.text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_product_and_protocol_literals() {
        let literals = protected_literals("ZeroClaw talks to LINE via MCP.");
        assert!(literals.iter().any(|literal| literal.text == "ZeroClaw"));
        assert!(literals.iter().any(|literal| literal.text == "LINE"));
        assert!(literals.iter().any(|literal| literal.text == "MCP"));
    }

    #[test]
    fn collects_machine_facing_inline_literals() {
        let literals =
            protected_literals("Run `zeroclaw daemon --profile <PROFILE>` with `ZEROCLAW_TEST`.");
        assert!(
            literals
                .iter()
                .any(|literal| literal.text == "zeroclaw daemon --profile <PROFILE>")
        );
        assert!(literals.iter().any(|literal| literal.text == "--profile"));
        assert!(
            literals
                .iter()
                .any(|literal| literal.text == "ZEROCLAW_TEST")
        );
    }

    #[test]
    fn collects_fenced_config_keys() {
        let literals = protected_literals(
            "```toml\n[observability]\nruntime_trace_path = \"state/runtime-trace.jsonl\"\n```\n\
             ```yaml\nallowed_private_hosts: []\n```\n\
             ```json\n{\"model_provider\": \"openai.default\"}\n```",
        );
        assert!(
            literals
                .iter()
                .any(|literal| literal.text == "[observability]")
        );
        assert!(
            literals
                .iter()
                .any(|literal| literal.text == "runtime_trace_path")
        );
        assert!(
            literals
                .iter()
                .any(|literal| literal.text == "allowed_private_hosts")
        );
        assert!(
            literals
                .iter()
                .any(|literal| literal.text == "model_provider")
        );
    }

    #[test]
    fn skips_plain_presentation_words() {
        let literals = protected_literals("Columns: `Area` | `Fix`.");
        assert!(!literals.iter().any(|literal| literal.text == "Area"));
        assert!(!literals.iter().any(|literal| literal.text == "Fix"));
    }

    #[test]
    fn reports_missing_literal_with_reason() {
        let missing = missing_protected_literal(
            "[`zeroclaw daemon`↴](#zeroclaw-daemon)",
            "[`zeroclaw 守护进程`↴](#zeroclaw-daemon)",
        )
        .expect("missing literal");
        assert_eq!(missing.text, "zeroclaw daemon");
        assert_eq!(missing.reason, "machine-facing code literal changed");
    }

    #[test]
    fn builds_preservation_prompt() {
        let prompt = preservation_prompt("Use `zeroclaw daemon` with ZeroClaw.")
            .expect("preservation prompt");
        assert!(prompt.contains("- ZeroClaw"));
        assert!(prompt.contains("- zeroclaw daemon"));
    }
}
