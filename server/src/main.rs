use actix_web::{error, post, web, App, HttpResponse, HttpServer, Responder, Error, Result};
use serde::{Serialize, Deserialize};
use futures::StreamExt;

// 256k bytes
const MAX_SIZE: usize = 262_144;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Alert {
    entry: f64,
    stop_loss: f64,
    trailing_pips: u32,
    trailing_offset: u32,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
          .service(alert)
          .route("/", web::get().to(test))
    })
      .bind(("lunar-engine.herokuapp.com", 8080))?
      .run()
      .await
}

#[post("/alert")]
async fn alert(mut payload: web::Payload) -> Result<HttpResponse, Error> {
    println!("Alert received");
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        println!("Chunk: {:?}", chunk);
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }
    let obj = serde_json::from_slice::<Alert>(&body)?;
    println!("Alert: {:?}", obj);
    Ok(HttpResponse::Ok().json(obj))
}

async fn test() -> impl Responder {
    HttpResponse::Ok().body("Server is running...")
}
