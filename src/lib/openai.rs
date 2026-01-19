use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    #[serde(default)]
    message: Option<OpenAIMessage>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    #[serde(default)]
    choices: Vec<OpenAIChoice>,
    #[serde(default)]
    error: Option<OpenAIError>,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    message: String,
}

pub struct OpenAIClient {
    api_key: String,
    model: String,
}

impl OpenAIClient {
    pub fn new() -> Result<Self, String> {
        dotenv::dotenv().ok();

        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY not found in .env".to_string())?;

        let model = env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4".to_string());

        Ok(OpenAIClient { api_key, model })
    }

    pub async fn generate_response(&self, messages: Vec<(String, String)>) -> Result<String, String> {
        let openai_messages: Vec<OpenAIMessage> = messages
            .into_iter()
            .map(|(persona, content)| OpenAIMessage {
                role: if persona == "User" {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                content,
            })
            .collect();

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
            temperature: 1.0,
        };

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to get response text: {}", e))?;

        let data: OpenAIResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse response: {} - Response: {}", e, text))?;

        // Check for API errors
        if let Some(err) = data.error {
            return Err(format!("OpenAI API error: {}", err.message));
        }

        // Extract response content
        data.choices
            .first()
            .and_then(|choice| choice.message.as_ref().map(|m| m.content.clone()))
            .or_else(|| data.choices.first().and_then(|choice| choice.text.clone()))
            .ok_or_else(|| format!("No response content from OpenAI. Status: {}", status))
    }
}
