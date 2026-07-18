use anyhow::{anyhow, Result};
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

// Embedding serialisation structures
#[derive(Serialize)]
struct EmbeddingContent {
    parts: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EmbedContentRequest {
    model: String,
    content: EmbeddingContent,
    output_dimensionality: i32,
}

#[derive(Serialize)]
struct EmbedBatchRequest {
    requests: Vec<EmbedContentRequest>,
}

#[derive(Deserialize)]
struct EmbeddingValues {
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbedContentResponse {
    embedding: EmbeddingValues,
}

#[derive(Deserialize)]
struct EmbedBatchResponse {
    embeddings: Vec<EmbeddingValues>,
}

fn get_api_key() -> Result<String> {
    env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow!("GEMINI_API_KEY environment variable not set"))
}

/// Generates a single 768-dimensional embedding vector for the given text.
pub async fn embed_text(text: &str) -> Result<Vec<f32>> {
    let api_key = get_api_key()?;
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent?key={}",
        api_key
    );

    let req = EmbedContentRequest {
        model: "models/gemini-embedding-001".to_string(),
        content: EmbeddingContent {
            parts: vec![ContentPart { text: text.to_string() }],
        },
        output_dimensionality: 768,
    };

    let res = client.post(&url).json(&req).send().await?;
    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini embedding API failed: {}", err_text));
    }

    let resp_body: EmbedContentResponse = res.json().await?;
    Ok(resp_body.embedding.values)
}

/// Generates multiple 768-dimensional embeddings in a single batch request (max 100 per batch).
pub async fn embed_texts(texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(vec![]);
    }

    let api_key = get_api_key()?;
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:batchEmbedContents?key={}",
        api_key
    );

    let mut all_embeddings = Vec::new();

    // Chunk into batches of up to 100 to stay within limits
    for chunk in texts.chunks(100) {
        let requests = chunk
            .iter()
            .map(|text| EmbedContentRequest {
                model: "models/gemini-embedding-001".to_string(),
                content: EmbeddingContent {
                    parts: vec![ContentPart { text: text.clone() }],
                },
                output_dimensionality: 768,
            })
            .collect::<Vec<_>>();

        let req = EmbedBatchRequest { requests };
        let res = client.post(&url).json(&req).send().await?;
        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini batch embedding API failed: {}", err_text));
        }

        let resp_body: EmbedBatchResponse = res.json().await?;
        for emb in resp_body.embeddings {
            all_embeddings.push(emb.values);
        }
    }

    Ok(all_embeddings)
}

/// Generates a response from gemini-3.5-flash given conversational history, active selection context, and retrieval chunks.
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
                parts: vec![ContentPart { text: text.to_string() }],
            });
        }
    }

    // Append the latest user query
    contents.push(Content {
        role: "user".to_string(),
        parts: vec![ContentPart { text: new_message_text.to_string() }],
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
