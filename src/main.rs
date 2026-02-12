mod client;

use anyhow::Result;
use client::OpenAIClient;
use client::dto::{ChatRequest, Message};

#[tokio::main]
async fn main() -> Result<()> {
    

    // Get API key from environment variable
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable must be set");
    let mut client = OpenAIClient::new(api_key);

    // Check for custom base URL
    if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
        client = client.with_base_url(base_url);
    }
    
    let request = ChatRequest {
        model: "openai/gpt-5.2".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: "This is a system_prompt".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "THis is a user_prompt".to_string(),
            },
        ],
        response_format: None,
        temperature: None,
        max_tokens: None,
        reasoning_effort: None,
    };

    let response = client.chat(request).await?;
    let output = response.choices[0].message.content.clone();

    println!("{}", output);

    Ok(())
}
