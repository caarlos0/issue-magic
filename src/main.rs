use anthropic::{
    client::Client,
    config::AnthropicConfig,
    types::{ContentBlock, Message, MessagesRequestBuilder, Role},
};
use anyhow::{Context, Result};
use clap::Parser;
use octocrab::Octocrab;
use serde::Deserialize;
use std::{collections::HashMap, env, fs};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    repository: Repository,
    labels: HashMap<String, Label>,
}

#[derive(Debug, Deserialize)]
struct Repository {
    owner: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Label {
    condition: String,
}

async fn list_issues(
    client: &Octocrab,
    owner: &str,
    repo: &str,
) -> Result<Vec<octocrab::models::issues::Issue>> {
    let issues = client
        .issues(owner, repo)
        .list()
        .labels(&[]) // Only get issues without labels
        .send()
        .await?;

    Ok(issues
        .items
        .into_iter()
        .filter(|issue| issue.pull_request.is_none())
        .collect())
}

fn build_prompt(issue: &octocrab::models::issues::Issue, config: &Config) -> String {
    let mut prompt = String::new();

    // Add issue information
    prompt.push_str("Based on the following issue, respond ONLY with a comma-separated list of labels that should be applied. If no labels apply, respond with 'none'.\n");
    prompt.push_str("If none of the label rules apply, respond with 'non'.\n\n");
    prompt.push_str(&format!("Issue Title: {}\n", issue.title));
    prompt.push_str(&format!(
        "Issue Body: {}\n\n",
        issue.body.as_deref().unwrap_or("")
    ));

    // Add label rules
    prompt.push_str("Labels and their conditions:\n");
    for (name, label) in &config.labels {
        prompt.push_str(&format!(
            "- Apply '{}' if the issue {}\n",
            name, label.condition
        ));
    }

    prompt
}

async fn ask_claude(prompt: &str) -> Result<Vec<String>> {
    let cfg = AnthropicConfig::new()?;
    let client = Client::try_from(cfg)?;

    let messages = vec![Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: prompt.into(),
        }],
    }];

    let messages_request = MessagesRequestBuilder::default()
        .messages(messages.clone())
        .model("claude-3-opus-20240229".to_string())
        .max_tokens(256usize)
        .build()?;

    let response = client.messages(messages_request).await?;

    //println!("messages response:\n\n{response:#?}");

    let labels = response
        .content
        .iter()
        .map(|c| match c {
            ContentBlock::Text { text } => text.clone(),
            _ => String::new(),
        })
        .collect::<Vec<String>>()
        .join("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| s != "none")
        .collect();

    Ok(labels)
}

async fn label_issue(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    issue_number: u64,
    labels: Vec<String>,
) -> Result<()> {
    if !labels.is_empty() {
        client
            .issues(owner, repo)
            .add_labels(issue_number, labels.as_slice())
            .await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config_content = fs::read_to_string(&args.config).context("Failed to read config file")?;
    let config: Config = toml::from_str(&config_content).context("Failed to parse config file")?;

    let github_token =
        env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable not set")?;
    let octocrab = Octocrab::builder().personal_token(github_token).build()?;

    let issues = list_issues(&octocrab, &config.repository.owner, &config.repository.name).await?;

    println!("Found {} unlabeled issues", issues.len());

    for issue in issues {
        println!("Processing issue #{} {}", issue.number, issue.title);

        let prompt = build_prompt(&issue, &config);
        let labels = ask_claude(&prompt).await?;

        if !labels.is_empty() {
            label_issue(
                &octocrab,
                &config.repository.owner,
                &config.repository.name,
                issue.number,
                labels.clone(),
            )
            .await?;
            println!("Added labels to issue #{}: {:?}", issue.number, labels);
        } else {
            println!("No labels to add for issue #{}", issue.number);
        }
    }

    Ok(())
}
