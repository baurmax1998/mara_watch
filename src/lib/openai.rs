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
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
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
            temperature: 0.7,
        };

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let data: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        data.choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| "No response from OpenAI".to_string())
    }
}
