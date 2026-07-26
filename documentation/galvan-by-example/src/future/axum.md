# Case Study: A Web API

The repository's *northstar* example (`example-projects/axum-api`) is a
complete ticket-tracking HTTP API on [axum](https://docs.rs/axum), written in
Galvan. It exercises nearly every chapter of this book at once — and marks
precisely where the language still falls short of its target.

> [!WARNING]
> **Aspirational.** The source below is the design target for the features
> annotated throughout this page. It does not fully transpile yet — the gaps
> are async functions, generic builder type propagation, and async-grade
> `ref` codegen.

Shared state is two `ref` fields; handlers are plain functions:

```galvan
use axum::Json
use axum::Router
use axum::extract::Path
use axum::extract::State
use axum::http::StatusCode
use axum::routing::get
use axum::routing::post
use std::net::SocketAddr

type CreateTicketRequest {
    title: String,
    priority: String?,
}

type TicketResponse {
    id: U64,
    title: String,
    priority: String,
    status: String,
    summary: String,
}

type ErrorResponse {
    error: String,
}

type ApiState {
    ref next_ticket_id: U64,
    ref tickets: [TicketResponse],
}
```

Routing and startup read like the axum original, minus the `Arc<Mutex<..>>`
and `#[tokio::main]` ceremony:

```galvan
async fn main() {
    let state = ApiState(
        next_ticket_id: 1,
        tickets: [],
    )

    let app = Router.new()
        .route("/health", get(health))
        .route("/tickets", get(list_tickets).post(create_ticket))
        .with_state(state)

    let addr = SocketAddr.from(([127, 0, 0, 1], 3000))
    println "listening on http://\(addr)"

    let listener = tokio::net::TcpListener.bind(addr).await!
    axum::serve(listener, app).await!
}
```

A handler mixes lifted axum types (`State`, `Json`, `StatusCode`) with
Galvan's collections, `ref` mutation, and string interpolation:

```galvan
async fn create_ticket(
    state: State<ApiState>,
    request: Json<CreateTicketRequest>,
) -> (StatusCode, Json<TicketResponse>) {
    let state = state.into_inner()
    let request = request.into_inner()

    let id = state.next_ticket_id
    state.next_ticket_id += 1

    let priority = request.priority else { "normal" }
    let response = TicketResponse(
        id: id,
        title: request.title,
        priority: priority,
        status: "open",
        summary: "[open] #\(id) \(request.title) (\(priority))",
    )

    state.tickets ++= response

    (StatusCode.CREATED, Json(response))
}
```

Failure paths use `throw` with a tuple of status code and error body:

```galvan
async fn get_ticket(
    state: State<ApiState>,
    id: Path<U64>,
) -> Result<Json<TicketResponse>, (StatusCode, Json<ErrorResponse>)> {
    let state = state.into_inner()
    let id = id.into_inner()

    for state.tickets |ticket| {
        if ticket.id == id {
            return Json(ticket)
        }
    }

    throw (
        StatusCode.NOT_FOUND,
        Json(ErrorResponse(error: "ticket \(id) was not found")),
    )
}
```

What this example demands from the language — and the state of each piece:

| Feature | Chapter | Status |
| --- | --- | --- |
| structs, optionals, interpolation | [Types](../types/index.md), [Errors](../errors/index.md) | ✅ implemented |
| `ref` fields for shared state | [Ownership](../ownership/ref_fields.md) | ⚠️ works, async-grade codegen missing |
| lifted axum/tokio APIs | [Interop](../interop/liftings.md) | ⚠️ functions/types lift; generic builder APIs not typechecked |
| `Router.new()`, `StatusCode.CREATED` | [Interop](../interop/methods.md) | ⚠️ syntax works; generic builder chains remain incomplete |
| `async fn`, `.await!` | [Async](async.md) | ❌ not implemented |

When this file transpiles and serves requests, Galvan 1.0 is close.
