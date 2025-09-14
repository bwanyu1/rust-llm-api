use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

pub async fn summarize(client: &Client, api_key: &str, model: &str, text: &str) -> Result<String> {
    let prompt = format!(
        "要約してください。重要な点を3〜5行で箇条書きにして、日本語で短く。\n\n本文:\n{}",
        text
    );

    let body = json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false
    });

    let url = "https://api.groq.com/openai/v1/chat/completions";
    let res = client
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("Groq API call failed (HTTP)")?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        anyhow::bail!("Groq API error: {} - {}", status, text);
    }

    let v: Value = res.json().await.context("failed to deserialize Groq JSON")?;
    let choices = v.get("choices")
        .and_then(|c| c.as_array())
        .context("Groq API response missing 'choices' array")?;
    let first_choice = choices.get(0)
        .context("Groq API response 'choices' array is empty")?;
    let message = first_choice.get("message")
        .context("Groq API response missing 'message' in first choice")?;
    let content = message.get("content")
        .and_then(|c| c.as_str())
        .context("Groq API response missing 'content' string in message")?;
    Ok(content.to_string())
}

