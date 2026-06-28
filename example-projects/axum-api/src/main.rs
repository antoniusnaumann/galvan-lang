use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

galvan::include!();

#[derive(Clone)]
struct ApiState {
    next_ticket_id: Arc<AtomicU64>,
    tickets: Arc<Mutex<Vec<TicketResponse>>>,
}

#[derive(Clone, Debug, Deserialize)]
struct CreateTicketRequest {
    title: String,
    priority: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct HealthResponse {
    service: String,
    status: String,
}

#[derive(Clone, Debug, Serialize)]
struct TicketResponse {
    id: u64,
    title: String,
    priority: String,
    status: String,
    summary: String,
}

#[derive(Clone, Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() {
    let state = ApiState {
        next_ticket_id: Arc::new(AtomicU64::new(1)),
        tickets: Arc::new(Mutex::new(Vec::new())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/tickets", get(list_tickets).post(create_ticket))
        .route("/tickets/{id}", get(get_ticket))
        .route("/tickets/{id}/close", post(close_ticket))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: galvan_module::service_name(),
        status: galvan_module::health_status(),
    })
}

async fn list_tickets(State(state): State<ApiState>) -> Json<Vec<TicketResponse>> {
    let tickets = state.tickets.lock().unwrap();
    Json(tickets.clone())
}

async fn create_ticket(
    State(state): State<ApiState>,
    Json(request): Json<CreateTicketRequest>,
) -> (StatusCode, Json<TicketResponse>) {
    let id = state.next_ticket_id.fetch_add(1, Ordering::Relaxed);
    let ticket = galvan_module::open_ticket(id, &request.title, &request.priority);
    let response = ticket_response(ticket);

    state.tickets.lock().unwrap().push(response.clone());

    (StatusCode::CREATED, Json(response))
}

async fn get_ticket(
    State(state): State<ApiState>,
    Path(id): Path<u64>,
) -> Result<Json<TicketResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tickets = state.tickets.lock().unwrap();
    tickets
        .iter()
        .find(|ticket| ticket.id == id)
        .cloned()
        .map(Json)
        .ok_or_else(|| not_found(id))
}

async fn close_ticket(
    State(state): State<ApiState>,
    Path(id): Path<u64>,
) -> Result<Json<TicketResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut tickets = state.tickets.lock().unwrap();
    let Some(ticket) = tickets.iter_mut().find(|ticket| ticket.id == id) else {
        return Err(not_found(id));
    };

    ticket.status = galvan_module::closed_status();
    ticket.summary =
        galvan_module::ticket_summary(ticket.id, &ticket.title, &ticket.priority, &ticket.status);

    Ok(Json(ticket.clone()))
}

fn ticket_response(ticket: galvan_module::Ticket) -> TicketResponse {
    let summary =
        galvan_module::ticket_summary(ticket.id, &ticket.title, &ticket.priority, &ticket.status);

    TicketResponse {
        id: ticket.id,
        title: ticket.title,
        priority: ticket.priority,
        status: ticket.status,
        summary,
    }
}

fn not_found(id: u64) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("ticket {id} was not found"),
        }),
    )
}
