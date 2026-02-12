pub mod dto;

use anyhow::Result;
use dto::{ChatRequest, ChatResponse};
#[cfg(test)]
use dto::{JsonSchema, Message, ResponseFormat};

pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error ({}): {}", status, error_text);
        }

        let chat_response = response.json::<ChatResponse>().await?;
        Ok(chat_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    #[tokio::test]
    async fn test_unstructured_output() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Mock the chat completions endpoint
        let mock_response = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 12,
                "total_tokens": 21
            }
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(matchers::header("authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        // Create client with mock server URL
        let client = OpenAIClient::new("test-api-key".to_string()).with_base_url(mock_server.uri());

        // Create a chat request without response_format (unstructured output)
        let request = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello!".to_string(),
            }],
            response_format: None,
            temperature: None,
            max_tokens: None,
            reasoning_effort: None,
        };

        // Send the request
        let response = client.chat(request).await.unwrap();

        // Verify the response
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            "Hello! How can I help you today?"
        );
        assert_eq!(response.choices[0].finish_reason, "stop");
        assert_eq!(response.usage.total_tokens, 21);
    }

    #[tokio::test]
    async fn test_structured_output() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Mock the chat completions endpoint with structured output
        let mock_response = serde_json::json!({
            "id": "chatcmpl-456",
            "object": "chat.completion",
            "created": 1677652290,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "{\"name\":\"John Doe\",\"age\":30,\"city\":\"New York\"}"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 20,
                "total_tokens": 35
            }
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(matchers::header("authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        // Create client with mock server URL
        let client = OpenAIClient::new("test-api-key".to_string()).with_base_url(mock_server.uri());

        // Create a JSON schema for structured output
        let json_schema = JsonSchema {
            name: "person".to_string(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "age": {"type": "number"},
                    "city": {"type": "string"}
                },
                "required": ["name", "age", "city"]
            }),
            strict: Some(true),
        };

        // Create a chat request with response_format (structured output)
        let request = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Tell me about a person".to_string(),
            }],
            response_format: Some(ResponseFormat {
                format_type: "json_schema".to_string(),
                json_schema: Some(json_schema),
            }),
            temperature: Some(0.7),
            max_tokens: Some(100),
            reasoning_effort: None,
        };

        // Send the request
        let response = client.chat(request).await.unwrap();

        // Verify the response
        assert_eq!(response.id, "chatcmpl-456");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            "{\"name\":\"John Doe\",\"age\":30,\"city\":\"New York\"}"
        );
        assert_eq!(response.choices[0].finish_reason, "stop");
        assert_eq!(response.usage.total_tokens, 35);
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Mock an error response
        let mock_error = serde_json::json!({
            "error": {
                "message": "Invalid API key",
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(matchers::header("authorization", "Bearer invalid-key"))
            .respond_with(ResponseTemplate::new(401).set_body_json(mock_error))
            .mount(&mock_server)
            .await;

        // Create client with mock server URL
        let client = OpenAIClient::new("invalid-key".to_string()).with_base_url(mock_server.uri());

        // Create a chat request
        let request = ChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello!".to_string(),
            }],
            response_format: None,
            temperature: None,
            max_tokens: None,
            reasoning_effort: None,
        };

        // Send the request and expect an error
        let result = client.chat(request).await;
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("401"));
    }
}
