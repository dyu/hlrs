use axum::{
  Router,
  http::{StatusCode, HeaderValue},
  response::{Response},
  middleware::{self, Next},
  extract::{Request},
};
use notify::Watcher;
use std::path::Path;
use tower_http::services::{ServeDir, ServeFile};
use tower_livereload::LiveReloadLayer;
use std::{env, format};

async fn insert_headers(req: Request, next: Next) -> Result<Response, StatusCode> {
  let mut res = next.run(req).await;
  let headers = res.headers_mut();
  headers.insert("Cache-Control", HeaderValue::from_static("no-cache, no-store, must-revalidate"));
  headers.insert("Pragma", HeaderValue::from_static("no-cache"));
  headers.insert("Cross-Origin-Embedder-Policy", HeaderValue::from_static("require-corp"));
  headers.insert("Cross-Origin-Opener-Policy", HeaderValue::from_static("same-origin"));
  Ok(res)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  let count = args.len() - 1;
  let port: u32 = if count != 0 {
    u32::from_str_radix(&args[1], 10).unwrap()
  } else {
    8080
  };
  let bind = format!("0.0.0.0:{port}");
  let livereload = LiveReloadLayer::new();
  let reloader = livereload.reloader();
  let mut r: Router = Router::new()
    .nest_service("/", ServeDir::new(Path::new(".")));
  
  let mut i = 2;
  while i < count {
    let entry: Vec<&str> = args[i].split(':').collect();
    if entry[1].ends_with(".html") {
      r = r.nest_service(entry[0], ServeFile::new(Path::new(entry[1])));
    } else {
      r = r.nest_service(entry[0], ServeDir::new(Path::new(entry[1])));
    }
    i += 1;
  }
  
  let app = r.layer(middleware::from_fn(insert_headers))
    .layer(livereload);
  
  let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
  watcher.watch(Path::new("."), notify::RecursiveMode::Recursive)?;
  let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
  axum::serve(listener, app).await.unwrap();
  Ok(())
}