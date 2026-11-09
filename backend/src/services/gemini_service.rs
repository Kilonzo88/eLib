use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize)]
struct ContentPart {
    text: String,
}

#[derive(Serialize)]
struct Content {
    role: String,
    parts: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInstruction {
    parts: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
}

#[derive(Deserialize)]
struct GenerateContentResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GenerateContentResponseCandidatesPart {
    parts: Vec<GenerateContentResponsePart>,
}

#[derive(Deserialize)]
struct GenerateContentResponseCandidate {
    content: GenerateContentResponseCandidatesPart,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<GenerateContentResponseCandidate>>,
}

fn get_api_key() -> Result<String> {
    env::var("GEMINI_API_KEY").map_err(|_| anyhow!("GEMINI_API_KEY environment variable not set"))
}

/// Generates a response from gemini-2.5-flash given conversational history, active selection context, and retrieval chunks.
pub async fn generate_chat(
    sys_instruction: &str,
    history: Vec<serde_json::Value>, // expect format: [{"role": "user"|"model", "text": "..."}]
    new_message_text: &str,
) -> Result<String> {
    let api_key = get_api_key()?;
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    // Map history to Gemini content structure
    let mut contents = Vec::new();
    for msg in history {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
        // Gemini API uses "model" instead of "assistant" for assistant responses
        let mapped_role = if role == "assistant" { "model" } else { role };

        let text = msg.get("text").and_then(|t| t.as_str()).unwrap_or("");
        if !text.is_empty() {
            contents.push(Content {
                role: mapped_role.to_string(),
                parts: vec![ContentPart {
                    text: text.to_string(),
                }],
            });
        }
    }

    // Append the latest user query
    contents.push(Content {
        role: "user".to_string(),
        parts: vec![ContentPart {
            text: new_message_text.to_string(),
        }],
    });

    let system_instruction = if !sys_instruction.is_empty() {
        Some(SystemInstruction {
            parts: vec![ContentPart {
                text: sys_instruction.to_string(),
            }],
        })
    } else {
        None
    };

    let req = GenerateContentRequest {
        contents,
        system_instruction,
    };

    let res = client.post(&url).json(&req).send().await?;
    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini Chat API failed: {}", err_text));
    }

    let resp_body: GenerateContentResponse = res.json().await?;
    let candidate = resp_body
        .candidates
        .and_then(|c| c.into_iter().next())
        .ok_or_else(|| anyhow!("Gemini returned no completions"))?;

    let text_reply = candidate
        .content
        .parts
        .into_iter()
        .filter_map(|p| p.text)
        .collect::<Vec<_>>()
        .join("\n");

    Ok(text_reply)
}
