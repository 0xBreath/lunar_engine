use std::str::FromStr;
use actix_web::{error, post, web, App, HttpResponse, HttpServer, Responder, Error, Result};
use serde::{Serialize, Deserialize};
use futures::StreamExt;

// 256k bytes
const MAX_SIZE: usize = 262_144;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Side {
    Long,
    Short
}
impl FromStr for Side {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Long" => Ok(Side::Long),
            "Short" => Ok(Side::Short),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Order {
    Enter,
    Exit
}
impl FromStr for Order {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Enter" => Ok(Order::Enter),
            "Exit" => Ok(Order::Exit),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Alert {
    side: Side,
    order: Order,
    timestamp: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    HttpServer::new(|| {
        App::new()
          .service(alert)
          .route("/", web::get().to(test))
    })
      .bind(bind_address)?
      .run()
      .await
}

#[post("/alert")]
async fn alert(mut payload: web::Payload) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        println!("Chunk: {:?}", chunk);
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }
    match serde_json::from_slice::<Alert>(&body) {
        Ok(alert) => {
            println!("{:?}", alert);
            let now = chrono::Utc::now().timestamp_millis();
            println!("Latency: {}ms", now - alert.timestamp.parse::<i64>().unwrap());
            Ok(HttpResponse::Ok().json(alert))
        },
        Err(e) => {
            println!("Error: {:?}", e);
            Err(error::ErrorBadRequest("invalid json"))
        }
    }
}

async fn test() -> impl Responder {
    HttpResponse::Ok().body("Server is running...")
}
