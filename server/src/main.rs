use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
struct Kanban {
    id: String,
    title: String,
    description: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
struct Column {
    id: String,
    kanban_id: String,
    title: String,
    position: i64,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
struct Card {
    id: String,
    column_id: String,
    title: String,
    description: String,
    position: i64,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateKanban {
    title: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateKanban {
    title: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateColumn {
    title: String,
}

#[derive(Debug, Deserialize)]
struct UpdateColumn {
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateCard {
    title: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateCard {
    title: Option<String>,
    description: Option<String>,
}

type AppState = Pool<Sqlite>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:kanban.db".to_string());
    let pool = SqlitePool::connect(&database_url).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS kanbans (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS columns (
            id TEXT PRIMARY KEY,
            kanban_id TEXT NOT NULL REFERENCES kanbans(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cards (
            id TEXT PRIMARY KEY,
            column_id TEXT NOT NULL REFERENCES columns(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await?;

    let app = Router::new()
        // Kanbans
        .route("/api/kanbans", get(list_kanbans).post(create_kanban))
        .route(
            "/api/kanbans/:id",
            get(get_kanban).put(update_kanban).delete(delete_kanban),
        )
        // Columns
        .route(
            "/api/kanbans/:kanban_id/columns",
            get(list_columns).post(create_column),
        )
        .route(
            "/api/columns/:id",
            get(get_column).put(update_column).delete(delete_column),
        )
        // Cards
        .route(
            "/api/columns/:column_id/cards",
            get(list_cards).post(create_card),
        )
        .route(
            "/api/cards/:id",
            get(get_card).put(update_card).delete(delete_card),
        )
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    println!("Server listening on http://0.0.0.0:3001");
    axum::serve(listener, app).await?;
    Ok(())
}

fn now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    secs.to_string()
}

// ---- Kanbans ----

async fn list_kanbans(State(pool): State<AppState>) -> Result<Json<Vec<Kanban>>, StatusCode> {
    let rows = sqlx::query_as::<_, Kanban>("SELECT * FROM kanbans ORDER BY created_at")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

async fn create_kanban(
    State(pool): State<AppState>,
    Json(body): Json<CreateKanban>,
) -> Result<(StatusCode, Json<Kanban>), StatusCode> {
    let id = Uuid::new_v4().to_string();
    let desc = body.description.unwrap_or_default();
    let created_at = now();
    sqlx::query("INSERT INTO kanbans (id, title, description, created_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(&body.title)
        .bind(&desc)
        .bind(&created_at)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let kanban = Kanban {
        id,
        title: body.title,
        description: desc,
        created_at,
    };
    Ok((StatusCode::CREATED, Json(kanban)))
}

async fn get_kanban(
    State(pool): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Kanban>, StatusCode> {
    let row = sqlx::query_as::<_, Kanban>("SELECT * FROM kanbans WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

async fn update_kanban(
    State(pool): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateKanban>,
) -> Result<Json<Kanban>, StatusCode> {
    let current = sqlx::query_as::<_, Kanban>("SELECT * FROM kanbans WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let title = body.title.unwrap_or(current.title);
    let description = body.description.unwrap_or(current.description);
    sqlx::query("UPDATE kanbans SET title = ?, description = ? WHERE id = ?")
        .bind(&title)
        .bind(&description)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let updated = Kanban {
        id,
        title,
        description,
        created_at: current.created_at,
    };
    Ok(Json(updated))
}

async fn delete_kanban(
    State(pool): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM kanbans WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- Columns ----

async fn list_columns(
    State(pool): State<AppState>,
    Path(kanban_id): Path<String>,
) -> Result<Json<Vec<Column>>, StatusCode> {
    let rows = sqlx::query_as::<_, Column>(
        "SELECT * FROM columns WHERE kanban_id = ? ORDER BY position, created_at",
    )
    .bind(&kanban_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

async fn create_column(
    State(pool): State<AppState>,
    Path(kanban_id): Path<String>,
    Json(body): Json<CreateColumn>,
) -> Result<(StatusCode, Json<Column>), StatusCode> {
    let max_pos: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(position), -1) FROM columns WHERE kanban_id = ?")
            .bind(&kanban_id)
            .fetch_one(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let position = max_pos.0 + 1;
    let id = Uuid::new_v4().to_string();
    let created_at = now();
    sqlx::query(
        "INSERT INTO columns (id, kanban_id, title, position, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&kanban_id)
    .bind(&body.title)
    .bind(position)
    .bind(&created_at)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let col = Column {
        id,
        kanban_id,
        title: body.title,
        position,
        created_at,
    };
    Ok((StatusCode::CREATED, Json(col)))
}

async fn get_column(
    State(pool): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Column>, StatusCode> {
    let row = sqlx::query_as::<_, Column>("SELECT * FROM columns WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

async fn update_column(
    State(pool): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateColumn>,
) -> Result<Json<Column>, StatusCode> {
    let current = sqlx::query_as::<_, Column>("SELECT * FROM columns WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let title = body.title.unwrap_or(current.title);
    sqlx::query("UPDATE columns SET title = ? WHERE id = ?")
        .bind(&title)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let updated = Column {
        id,
        title,
        ..current
    };
    Ok(Json(updated))
}

async fn delete_column(
    State(pool): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM columns WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- Cards ----

async fn list_cards(
    State(pool): State<AppState>,
    Path(column_id): Path<String>,
) -> Result<Json<Vec<Card>>, StatusCode> {
    let rows = sqlx::query_as::<_, Card>(
        "SELECT * FROM cards WHERE column_id = ? ORDER BY position, created_at",
    )
    .bind(&column_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

async fn create_card(
    State(pool): State<AppState>,
    Path(column_id): Path<String>,
    Json(body): Json<CreateCard>,
) -> Result<(StatusCode, Json<Card>), StatusCode> {
    let max_pos: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(position), -1) FROM cards WHERE column_id = ?")
            .bind(&column_id)
            .fetch_one(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let position = max_pos.0 + 1;
    let id = Uuid::new_v4().to_string();
    let desc = body.description.unwrap_or_default();
    let created_at = now();
    sqlx::query("INSERT INTO cards (id, column_id, title, description, position, created_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(&column_id).bind(&body.title).bind(&desc).bind(position).bind(&created_at)
        .execute(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let card = Card {
        id,
        column_id,
        title: body.title,
        description: desc,
        position,
        created_at,
    };
    Ok((StatusCode::CREATED, Json(card)))
}

async fn get_card(
    State(pool): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Card>, StatusCode> {
    let row = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

async fn update_card(
    State(pool): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCard>,
) -> Result<Json<Card>, StatusCode> {
    let current = sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
        .bind(&id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let title = body.title.unwrap_or(current.title);
    let description = body.description.unwrap_or(current.description);
    sqlx::query("UPDATE cards SET title = ?, description = ? WHERE id = ?")
        .bind(&title)
        .bind(&description)
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let updated = Card {
        id,
        title,
        description,
        ..current
    };
    Ok(Json(updated))
}

async fn delete_card(
    State(pool): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM cards WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
