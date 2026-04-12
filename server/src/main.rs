use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use moka::sync::Cache;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

const CACHE_TTL_SECS: u64 = 30;
const KANBAN_LIST_KEY: &str = "kanbans:list";

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

#[derive(Clone)]
struct AppState {
    pool: Pool<Sqlite>,
    cache: AppCache,
}

#[derive(Clone)]
struct AppCache {
    kanban_list: Cache<String, Vec<Kanban>>,
    kanbans: Cache<String, Kanban>,
    columns_by_kanban: Cache<String, Vec<Column>>,
    columns: Cache<String, Column>,
    cards_by_column: Cache<String, Vec<Card>>,
    cards: Cache<String, Card>,
}

impl AppState {
    fn new(pool: Pool<Sqlite>) -> Self {
        let ttl = Duration::from_secs(CACHE_TTL_SECS);
        Self {
            pool,
            cache: AppCache {
                kanban_list: Cache::builder().time_to_live(ttl).max_capacity(64).build(),
                kanbans: Cache::builder()
                    .time_to_live(ttl)
                    .max_capacity(1_024)
                    .build(),
                columns_by_kanban: Cache::builder().time_to_live(ttl).max_capacity(256).build(),
                columns: Cache::builder()
                    .time_to_live(ttl)
                    .max_capacity(2_048)
                    .build(),
                cards_by_column: Cache::builder()
                    .time_to_live(ttl)
                    .max_capacity(1_024)
                    .build(),
                cards: Cache::builder()
                    .time_to_live(ttl)
                    .max_capacity(8_192)
                    .build(),
            },
        }
    }

    fn invalidate_kanban_list(&self) {
        self.cache.kanban_list.invalidate(KANBAN_LIST_KEY);
    }

    fn invalidate_kanban(&self, kanban_id: &str) {
        self.cache.kanbans.invalidate(kanban_id);
    }

    fn invalidate_column_list(&self, kanban_id: &str) {
        self.cache.columns_by_kanban.invalidate(kanban_id);
    }

    fn invalidate_column(&self, column_id: &str) {
        self.cache.columns.invalidate(column_id);
    }

    fn invalidate_card_list(&self, column_id: &str) {
        self.cache.cards_by_column.invalidate(column_id);
    }

    fn invalidate_card(&self, card_id: &str) {
        self.cache.cards.invalidate(card_id);
    }

    fn invalidate_column_tree(&self, column: &Column) {
        self.invalidate_column_list(&column.kanban_id);
        self.invalidate_column(&column.id);
    }

    fn invalidate_card_tree(&self, card: &Card) {
        self.invalidate_card_list(&card.column_id);
        self.invalidate_card(&card.id);
    }
}

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
        .route("/api/kanbans", get(list_kanbans).post(create_kanban))
        .route(
            "/api/kanbans/:id",
            get(get_kanban).put(update_kanban).delete(delete_kanban),
        )
        .route(
            "/api/kanbans/:kanban_id/columns",
            get(list_columns).post(create_column),
        )
        .route(
            "/api/columns/:id",
            get(get_column).put(update_column).delete(delete_column),
        )
        .route(
            "/api/columns/:column_id/cards",
            get(list_cards).post(create_card),
        )
        .route(
            "/api/cards/:id",
            get(get_card).put(update_card).delete(delete_card),
        )
        .layer(CorsLayer::permissive())
        .with_state(AppState::new(pool));

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

fn map_fetch_one_error(error: sqlx::Error) -> StatusCode {
    match error {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn load_kanbans(pool: &Pool<Sqlite>) -> Result<Vec<Kanban>, StatusCode> {
    sqlx::query_as::<_, Kanban>("SELECT * FROM kanbans ORDER BY created_at")
        .fetch_all(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn load_kanban(pool: &Pool<Sqlite>, id: &str) -> Result<Kanban, StatusCode> {
    sqlx::query_as::<_, Kanban>("SELECT * FROM kanbans WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_fetch_one_error)
}

async fn load_columns(pool: &Pool<Sqlite>, kanban_id: &str) -> Result<Vec<Column>, StatusCode> {
    sqlx::query_as::<_, Column>(
        "SELECT * FROM columns WHERE kanban_id = ? ORDER BY position, created_at",
    )
    .bind(kanban_id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn load_column(pool: &Pool<Sqlite>, id: &str) -> Result<Column, StatusCode> {
    sqlx::query_as::<_, Column>("SELECT * FROM columns WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_fetch_one_error)
}

async fn load_cards(pool: &Pool<Sqlite>, column_id: &str) -> Result<Vec<Card>, StatusCode> {
    sqlx::query_as::<_, Card>(
        "SELECT * FROM cards WHERE column_id = ? ORDER BY position, created_at",
    )
    .bind(column_id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn load_card(pool: &Pool<Sqlite>, id: &str) -> Result<Card, StatusCode> {
    sqlx::query_as::<_, Card>("SELECT * FROM cards WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(map_fetch_one_error)
}

async fn load_cards_for_kanban(
    pool: &Pool<Sqlite>,
    kanban_id: &str,
) -> Result<Vec<Card>, StatusCode> {
    sqlx::query_as::<_, Card>(
        "SELECT cards.* FROM cards \
         INNER JOIN columns ON columns.id = cards.column_id \
         WHERE columns.kanban_id = ?",
    )
    .bind(kanban_id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ---- Kanbans ----

async fn list_kanbans(State(state): State<AppState>) -> Result<Json<Vec<Kanban>>, StatusCode> {
    if let Some(rows) = state.cache.kanban_list.get(KANBAN_LIST_KEY) {
        return Ok(Json(rows));
    }

    let rows = load_kanbans(&state.pool).await?;
    state
        .cache
        .kanban_list
        .insert(KANBAN_LIST_KEY.to_string(), rows.clone());
    Ok(Json(rows))
}

async fn create_kanban(
    State(state): State<AppState>,
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
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let kanban = Kanban {
        id,
        title: body.title,
        description: desc,
        created_at,
    };
    state.invalidate_kanban_list();
    state
        .cache
        .kanbans
        .insert(kanban.id.clone(), kanban.clone());
    Ok((StatusCode::CREATED, Json(kanban)))
}

async fn get_kanban(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Kanban>, StatusCode> {
    if let Some(row) = state.cache.kanbans.get(&id) {
        return Ok(Json(row));
    }

    let row = load_kanban(&state.pool, &id).await?;
    state.cache.kanbans.insert(id, row.clone());
    Ok(Json(row))
}

async fn update_kanban(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateKanban>,
) -> Result<Json<Kanban>, StatusCode> {
    let current = load_kanban(&state.pool, &id).await?;
    let title = body.title.unwrap_or_else(|| current.title.clone());
    let description = body
        .description
        .unwrap_or_else(|| current.description.clone());

    sqlx::query("UPDATE kanbans SET title = ?, description = ? WHERE id = ?")
        .bind(&title)
        .bind(&description)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let updated = Kanban {
        id: id.clone(),
        title,
        description,
        created_at: current.created_at,
    };
    state.invalidate_kanban_list();
    state.cache.kanbans.insert(id, updated.clone());
    Ok(Json(updated))
}

async fn delete_kanban(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    load_kanban(&state.pool, &id).await?;
    let columns = load_columns(&state.pool, &id).await?;
    let cards = load_cards_for_kanban(&state.pool, &id).await?;

    let result = sqlx::query("DELETE FROM kanbans WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state.invalidate_kanban_list();
    state.invalidate_kanban(&id);
    state.invalidate_column_list(&id);
    for column in &columns {
        state.invalidate_column(&column.id);
        state.invalidate_card_list(&column.id);
    }
    for card in &cards {
        state.invalidate_card(&card.id);
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---- Columns ----

async fn list_columns(
    State(state): State<AppState>,
    Path(kanban_id): Path<String>,
) -> Result<Json<Vec<Column>>, StatusCode> {
    if let Some(rows) = state.cache.columns_by_kanban.get(&kanban_id) {
        return Ok(Json(rows));
    }

    let rows = load_columns(&state.pool, &kanban_id).await?;
    state
        .cache
        .columns_by_kanban
        .insert(kanban_id, rows.clone());
    Ok(Json(rows))
}

async fn create_column(
    State(state): State<AppState>,
    Path(kanban_id): Path<String>,
    Json(body): Json<CreateColumn>,
) -> Result<(StatusCode, Json<Column>), StatusCode> {
    let max_pos: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(position), -1) FROM columns WHERE kanban_id = ?")
            .bind(&kanban_id)
            .fetch_one(&state.pool)
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
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let column = Column {
        id,
        kanban_id: kanban_id.clone(),
        title: body.title,
        position,
        created_at,
    };
    state.invalidate_column_list(&kanban_id);
    state
        .cache
        .columns
        .insert(column.id.clone(), column.clone());
    Ok((StatusCode::CREATED, Json(column)))
}

async fn get_column(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Column>, StatusCode> {
    if let Some(row) = state.cache.columns.get(&id) {
        return Ok(Json(row));
    }

    let row = load_column(&state.pool, &id).await?;
    state.cache.columns.insert(id, row.clone());
    Ok(Json(row))
}

async fn update_column(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateColumn>,
) -> Result<Json<Column>, StatusCode> {
    let current = load_column(&state.pool, &id).await?;
    let title = body.title.unwrap_or_else(|| current.title.clone());

    sqlx::query("UPDATE columns SET title = ? WHERE id = ?")
        .bind(&title)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let updated = Column {
        id: id.clone(),
        kanban_id: current.kanban_id.clone(),
        title,
        position: current.position,
        created_at: current.created_at,
    };
    state.invalidate_column_tree(&updated);
    state.cache.columns.insert(id, updated.clone());
    Ok(Json(updated))
}

async fn delete_column(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let column = load_column(&state.pool, &id).await?;
    let cards = load_cards(&state.pool, &id).await?;

    let result = sqlx::query("DELETE FROM columns WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state.invalidate_column_tree(&column);
    state.invalidate_card_list(&id);
    for card in &cards {
        state.invalidate_card(&card.id);
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---- Cards ----

async fn list_cards(
    State(state): State<AppState>,
    Path(column_id): Path<String>,
) -> Result<Json<Vec<Card>>, StatusCode> {
    if let Some(rows) = state.cache.cards_by_column.get(&column_id) {
        return Ok(Json(rows));
    }

    let rows = load_cards(&state.pool, &column_id).await?;
    state.cache.cards_by_column.insert(column_id, rows.clone());
    Ok(Json(rows))
}

async fn create_card(
    State(state): State<AppState>,
    Path(column_id): Path<String>,
    Json(body): Json<CreateCard>,
) -> Result<(StatusCode, Json<Card>), StatusCode> {
    let max_pos: (i64,) =
        sqlx::query_as("SELECT COALESCE(MAX(position), -1) FROM cards WHERE column_id = ?")
            .bind(&column_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let position = max_pos.0 + 1;
    let id = Uuid::new_v4().to_string();
    let description = body.description.unwrap_or_default();
    let created_at = now();

    sqlx::query(
        "INSERT INTO cards (id, column_id, title, description, position, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&column_id)
    .bind(&body.title)
    .bind(&description)
    .bind(position)
    .bind(&created_at)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let card = Card {
        id,
        column_id: column_id.clone(),
        title: body.title,
        description,
        position,
        created_at,
    };
    state.invalidate_card_list(&column_id);
    state.cache.cards.insert(card.id.clone(), card.clone());
    Ok((StatusCode::CREATED, Json(card)))
}

async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Card>, StatusCode> {
    if let Some(row) = state.cache.cards.get(&id) {
        return Ok(Json(row));
    }

    let row = load_card(&state.pool, &id).await?;
    state.cache.cards.insert(id, row.clone());
    Ok(Json(row))
}

async fn update_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCard>,
) -> Result<Json<Card>, StatusCode> {
    let current = load_card(&state.pool, &id).await?;
    let title = body.title.unwrap_or_else(|| current.title.clone());
    let description = body
        .description
        .unwrap_or_else(|| current.description.clone());

    sqlx::query("UPDATE cards SET title = ?, description = ? WHERE id = ?")
        .bind(&title)
        .bind(&description)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let updated = Card {
        id: id.clone(),
        column_id: current.column_id.clone(),
        title,
        description,
        position: current.position,
        created_at: current.created_at,
    };
    state.invalidate_card_tree(&updated);
    state.cache.cards.insert(id, updated.clone());
    Ok(Json(updated))
}

async fn delete_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let card = load_card(&state.pool, &id).await?;

    let result = sqlx::query("DELETE FROM cards WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state.invalidate_card_tree(&card);
    Ok(StatusCode::NO_CONTENT)
}
