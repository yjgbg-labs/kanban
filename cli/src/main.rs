use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "kanban", about = "Kanban CLI - manage kanbans, columns, and cards")]
struct Cli {
    /// Server URL (default: http://localhost:3001)
    #[arg(long, env = "SERVER_URL", default_value = "http://localhost:3001")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage kanbans
    Kanban {
        #[command(subcommand)]
        action: KanbanAction,
    },
    /// Manage columns
    Column {
        #[command(subcommand)]
        action: ColumnAction,
    },
    /// Manage cards
    Card {
        #[command(subcommand)]
        action: CardAction,
    },
}

#[derive(Subcommand)]
enum KanbanAction {
    /// List all kanbans
    List,
    /// Get a kanban by ID
    Get { id: String },
    /// Create a new kanban
    Create {
        #[arg(short, long)]
        title: String,
        #[arg(short, long, default_value = "")]
        description: String,
    },
    /// Update a kanban
    Update {
        id: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a kanban
    Delete { id: String },
}

#[derive(Subcommand)]
enum ColumnAction {
    /// List columns in a kanban
    List { kanban_id: String },
    /// Get a column by ID
    Get { id: String },
    /// Create a column in a kanban
    Create {
        kanban_id: String,
        #[arg(short, long)]
        title: String,
    },
    /// Update a column
    Update {
        id: String,
        #[arg(short, long)]
        title: Option<String>,
    },
    /// Delete a column
    Delete { id: String },
}

#[derive(Subcommand)]
enum CardAction {
    /// List cards in a column
    List { column_id: String },
    /// Get a card by ID
    Get { id: String },
    /// Create a card in a column
    Create {
        column_id: String,
        #[arg(short, long)]
        title: String,
        #[arg(short, long, default_value = "")]
        description: String,
    },
    /// Update a card
    Update {
        id: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a card
    Delete { id: String },
}

#[derive(Serialize)]
struct CreateKanbanReq {
    title: String,
    description: String,
}

#[derive(Serialize)]
struct UpdateKanbanReq {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize)]
struct CreateColumnReq {
    title: String,
}

#[derive(Serialize)]
struct UpdateColumnReq {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Serialize)]
struct CreateCardReq {
    title: String,
    description: String,
}

#[derive(Serialize)]
struct UpdateCardReq {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    #[serde(default)]
    message: String,
}

fn print_json(value: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

fn check_response(resp: reqwest::blocking::Response) -> Result<serde_json::Value> {
    let status = resp.status();
    if status.is_success() {
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::Value::Null);
        }
        let body: serde_json::Value = resp.json().context("Failed to parse response")?;
        return Ok(body);
    }
    let text = resp.text().unwrap_or_default();
    if let Ok(err) = serde_json::from_str::<ApiError>(&text) {
        anyhow::bail!("{}: {}", status, err.message);
    }
    anyhow::bail!("{}: {}", status, text);
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = reqwest::blocking::Client::new();
    let base = cli.server.trim_end_matches('/');

    match cli.command {
        Commands::Kanban { action } => match action {
            KanbanAction::List => {
                let resp = client.get(format!("{base}/api/kanbans")).send()?;
                print_json(&check_response(resp)?);
            }
            KanbanAction::Get { id } => {
                let resp = client.get(format!("{base}/api/kanbans/{id}")).send()?;
                print_json(&check_response(resp)?);
            }
            KanbanAction::Create { title, description } => {
                let resp = client
                    .post(format!("{base}/api/kanbans"))
                    .json(&CreateKanbanReq { title, description })
                    .send()?;
                print_json(&check_response(resp)?);
            }
            KanbanAction::Update {
                id,
                title,
                description,
            } => {
                let resp = client
                    .put(format!("{base}/api/kanbans/{id}"))
                    .json(&UpdateKanbanReq { title, description })
                    .send()?;
                print_json(&check_response(resp)?);
            }
            KanbanAction::Delete { id } => {
                let resp = client.delete(format!("{base}/api/kanbans/{id}")).send()?;
                check_response(resp)?;
                println!("Deleted.");
            }
        },
        Commands::Column { action } => match action {
            ColumnAction::List { kanban_id } => {
                let resp = client
                    .get(format!("{base}/api/kanbans/{kanban_id}/columns"))
                    .send()?;
                print_json(&check_response(resp)?);
            }
            ColumnAction::Get { id } => {
                let resp = client.get(format!("{base}/api/columns/{id}")).send()?;
                print_json(&check_response(resp)?);
            }
            ColumnAction::Create { kanban_id, title } => {
                let resp = client
                    .post(format!("{base}/api/kanbans/{kanban_id}/columns"))
                    .json(&CreateColumnReq { title })
                    .send()?;
                print_json(&check_response(resp)?);
            }
            ColumnAction::Update { id, title } => {
                let resp = client
                    .put(format!("{base}/api/columns/{id}"))
                    .json(&UpdateColumnReq { title })
                    .send()?;
                print_json(&check_response(resp)?);
            }
            ColumnAction::Delete { id } => {
                let resp = client.delete(format!("{base}/api/columns/{id}")).send()?;
                check_response(resp)?;
                println!("Deleted.");
            }
        },
        Commands::Card { action } => match action {
            CardAction::List { column_id } => {
                let resp = client
                    .get(format!("{base}/api/columns/{column_id}/cards"))
                    .send()?;
                print_json(&check_response(resp)?);
            }
            CardAction::Get { id } => {
                let resp = client.get(format!("{base}/api/cards/{id}")).send()?;
                print_json(&check_response(resp)?);
            }
            CardAction::Create {
                column_id,
                title,
                description,
            } => {
                let resp = client
                    .post(format!("{base}/api/columns/{column_id}/cards"))
                    .json(&CreateCardReq { title, description })
                    .send()?;
                print_json(&check_response(resp)?);
            }
            CardAction::Update {
                id,
                title,
                description,
            } => {
                let resp = client
                    .put(format!("{base}/api/cards/{id}"))
                    .json(&UpdateCardReq { title, description })
                    .send()?;
                print_json(&check_response(resp)?);
            }
            CardAction::Delete { id } => {
                let resp = client.delete(format!("{base}/api/cards/{id}")).send()?;
                check_response(resp)?;
                println!("Deleted.");
            }
        },
    }

    Ok(())
}
