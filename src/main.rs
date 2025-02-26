use axum::Router;
use bihelix_rgb_cli::{post, Json, Method};
use bihelix_rgb_cli::{IssueRgb20Request, IssueRgb20Response};
use tower_http::cors::{Any, CorsLayer};

async fn issue_rgb20_service(
    Json(payload): Json<IssueRgb20Request>,
) -> Result<Json<IssueRgb20Response>, String> {
    eprintln!("Received an issue rgb20 inflation contract request");
    let ticker = payload.ticker;
    let name = payload.name;
    let precision = payload.precision;
    let issued_supply = payload.issued_supply;
    let allocate_outpoint = payload.allocate_outpoint;

    let rgb20_contract_id = bihelix_rgb_cli::issue_rgb20(
        &ticker,
        &name,
        precision,
        issued_supply,
        &allocate_outpoint,
        &payload.electrum_url,
    )
    .await?;
    Ok(Json(IssueRgb20Response { rgb20_contract_id }))
}

use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Listen address
    #[arg(short = 'a', long = "host", default_value = "0.0.0.0")]
    host: String,

    /// Listen port
    #[arg(short = 'p', long = "port", default_value = "3000")]
    port: u16,
}

#[tokio::main]
pub async fn main() -> Result<(), String> {
    bihelix_rgb_cli::generate_wallet().await;
    eprintln!("generate wallet done");

    let cors = CorsLayer::new()
        .allow_origin(Any) // allow all origins
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ]) // allow all common HTTP method
        .allow_headers(Any);

    let args = Args::parse();
    let addr = SocketAddr::new(args.host.parse().unwrap(), args.port);
    // set router
    let app = Router::new()
        .route("/issuergb20", post(issue_rgb20_service))
        .layer(cors);

    // start service
    // let addr = "0.0.0.0:3000";
    eprintln!("Server running at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {:?}", e);
    };
    Ok(())
}
