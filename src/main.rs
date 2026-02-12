mod client;
mod git;
mod prompt;
mod tools;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

use client::dto::{ChatRequest, Message};
use client::OpenAIClient;
use git::get_git_data;
use prompt::{create_user_prompt, get_system_prompt};
use tools::tool_definitions;

const DEFAULT_MODEL: &str = "openai/gpt-5.2";
const MAX_TOOL_CALLS: usize = 8;

#[derive(Parser, Debug)]
#[command(name = "blart")]
#[command(about = "AI-powered code review tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a code review on the current git branch
    Review(ReviewArgs),
}

#[derive(Parser, Debug)]
struct ReviewArgs {
    /// Default branch name to compare against
    #[arg(long, default_value = "main")]
    default_branch: String,

    /// If set, do not make any changes, just print what would be done
    #[arg(long)]
    dry_run: bool,

    /// OpenAI API key (if not provided, will use OPENAI_API_KEY environment variable)
    #[arg(long)]
    api_key: Option<String>,

    /// Additional context to add to the user prompt
    #[arg(long, default_value = "")]
    additional_prompt: String,

    /// Reasoning effort level
    #[arg(
        long,
        default_value = "high",
        value_parser = ["none", "minimal", "low", "medium", "high", "xhigh"]
    )]
    reasoning_effort: String,

    /// OpenAI model to use for the review
    #[arg(long, default_value = DEFAULT_MODEL)]
    model: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Review(args) => run_review(args).await,
    }
}

async fn run_review(args: ReviewArgs) -> Result<()> {
    let git_data = get_git_data(&args.default_branch)?;

    if git_data.diff.trim().is_empty() {
        println!("No changes detected.");
        return Ok(());
    }
    if git_data.files_changed.is_empty() {
        println!("No changed files detected.");
        return Ok(());
    }

    let system_prompt = get_system_prompt();
    let additional_prompt = if args.additional_prompt.trim().is_empty() {
        None
    } else {
        Some(args.additional_prompt.as_str())
    };
    let user_prompt = create_user_prompt(&git_data.diff, &git_data.files_changed, additional_prompt);

    if args.dry_run {
        println!("System prompt:\n{}", system_prompt);
        println!("\nUser prompt:\n{}", user_prompt);
        println!("\nModel: {}", args.model);
        println!("Reasoning effort: {}", args.reasoning_effort);
        return Ok(());
    }

    let api_key = args
        .api_key
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
        .context("OpenAI API key must be provided via --api-key argument or OPENAI_API_KEY environment variable")?;

    let mut client = OpenAIClient::new(api_key);
    if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
        client = client.with_base_url(base_url);
    }

    let tools = tool_definitions();
    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: Some(system_prompt),
            tool_calls: None,
            tool_call_id: None,
        },
        Message {
            role: "user".to_string(),
            content: Some(user_prompt),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let mut tool_calls_used = 0;
    loop {
        let request = ChatRequest {
            model: args.model.clone(),
            messages: messages.clone(),
            response_format: None,
            tools: Some(tools.clone()),
            tool_choice: Some("auto".to_string()),
            temperature: None,
            max_tokens: None,
            reasoning_effort: Some(args.reasoning_effort.clone()),
        };

        let response = client.chat(request).await?;
        let choice = response
            .choices
            .into_iter()
            .next()
            .context("No response choices returned")?;
        let assistant_message = choice.message;
        let tool_calls = assistant_message.tool_calls.clone();

        messages.push(assistant_message.clone());

        if let Some(tool_calls) = tool_calls {
            for call in tool_calls {
                tool_calls_used += 1;
                if tool_calls_used > MAX_TOOL_CALLS {
                    return Err(anyhow!(
                        "Tool call limit exceeded (max {}).", MAX_TOOL_CALLS
                    ));
                }

                let summary = tools::summarize_tool_call(
                    &call.function.name,
                    &call.function.arguments,
                );
                println!("Tool call: {}", summary);

                let tool_output =
                    tools::handle_tool_call(&call.function.name, &call.function.arguments);

                messages.push(Message {
                    role: "tool".to_string(),
                    content: Some(tool_output),
                    tool_calls: None,
                    tool_call_id: Some(call.id),
                });
            }
            continue;
        }

        let content = assistant_message.content.unwrap_or("<no content>".to_string());
        if content.trim().is_empty() || content == "<no content>" {
            return Err(anyhow!(
                "Model returned an empty response with no tool calls."
            ));
        }
        println!("{}", content.trim());
        break;
    }

    Ok(())
}
