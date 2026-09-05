use futures::StreamExt;

use crate::provider::{self, ProviderConfig};

pub fn normalize_title(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let collapsed = trimmed
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let compact = collapsed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let sanitized = compact
        .trim_matches('"')
        .trim_matches('\'')
        .trim();

    let mut cleaned = sanitized.to_string();
    if cleaned.chars().count() > 48 {
        cleaned = cleaned.chars().take(48).collect();
        while !cleaned.is_empty()
            && cleaned
                .chars()
                .last()
                .expect("checked is_empty")
                .is_whitespace()
        {
            cleaned.pop();
        }
    }

    cleaned
}

pub async fn generate_title(
    config: &ProviderConfig,
    model: &str,
    prompt: &str,
) -> anyhow::Result<String> {
    let instruction = format!(
        "Сгенерируй очень короткое и понятное название для чата на русском языке. Не используй кавычки, не пиши пояснений, только сам заголовок. Тема: {}",
        prompt.trim()
    );

    let history = Vec::new();
    let stream_result =
        provider::start_stream(config, model, instruction, history, None, 1).await;

    let (mut events, _control) = stream_result?;
    let mut text = String::new();

    while let Some(event) = events.next().await {
        match event {
            provider::ChatEvent::Delta(delta) => text.push_str(&delta),
            provider::ChatEvent::Done { text: done, .. } => {
                text = done;
                break;
            },
            provider::ChatEvent::Error(err) => anyhow::bail!(err),
            provider::ChatEvent::Reasoning(_)
            | provider::ChatEvent::ToolCallStarted { .. }
            | provider::ChatEvent::ToolResultReceived { .. }
            | provider::ChatEvent::Usage(_) => {},
        }
    }

    let normalized = normalize_title(&text);
    if normalized.is_empty() {
        anyhow::bail!("empty title")
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_title_strips_quotes_and_newlines() {
        assert_eq!(
            normalize_title("\n  \"Срочный запрос\"  \n"),
            "Срочный запрос"
        );
        assert_eq!(
            normalize_title("Много   пробелов\nи\nпереносов"),
            "Много пробелов и переносов"
        );
        assert_eq!(
            normalize_title("x".repeat(60).as_str()),
            "x".repeat(48)
        );
    }
}
