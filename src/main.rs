use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use axum::{
  Router,
  http::{StatusCode, HeaderValue},
  response::{Response},
  middleware::{self, Next},
  extract::{Request},
};
use notify::Watcher;
use std::{env, format, fs, path::Path};
use tower_http::services::{ServeDir, ServeFile};
use tower_livereload::LiveReloadLayer;

fn is_truthy(str: String) -> bool {
  str == "1" || str == "true"
}

async fn insert_watch_headers(req: Request, next: Next) -> Result<Response, StatusCode> {
  let mut res = next.run(req).await;
  let headers = res.headers_mut();
  headers.insert("Cache-Control", HeaderValue::from_static("no-cache, no-store, must-revalidate"));
  headers.insert("Pragma", HeaderValue::from_static("no-cache"));
  headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
  headers.insert("Cross-Origin-Embedder-Policy", HeaderValue::from_static("require-corp"));
  headers.insert("Cross-Origin-Opener-Policy", HeaderValue::from_static("same-origin"));
  Ok(res)
}

async fn insert_serve_headers(req: Request, next: Next) -> Result<Response, StatusCode> {
  let mut res = next.run(req).await;
  let headers = res.headers_mut();
  headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
  headers.insert("Cross-Origin-Embedder-Policy", HeaderValue::from_static("require-corp"));
  headers.insert("Cross-Origin-Opener-Policy", HeaderValue::from_static("same-origin"));
  Ok(res)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  let count = args.len();
  let offset = 1;
  let port: u32 = if count > offset {
    u32::from_str_radix(&args[offset], 10).unwrap()
  } else {
    8080
  };
  let bind = format!("0.0.0.0:{port}");
  let base_dir = fs::canonicalize(".")?;
  
  let mut r = Router::new();
  let mut i = offset + 1;
  
  if i == count || !args[i].starts_with("/:") {
    r = r.nest_service("/", ServeDir::new(base_dir.as_path())
        .not_found_service(ServeFile::new(Path::new("./index.html"))));
  }
  
  while i < count {
    let entry: Vec<&str> = args[i].split(':').collect();
    if !entry[1].ends_with(".html") {
      r = r.nest_service(entry[0], ServeDir::new(Path::new(entry[1])));
    } else if entry[0].ends_with("/") {
      r = r.nest_service(entry[0], ServeFile::new(Path::new(entry[1])));
    } else {
      r = r.route_service(entry[0], ServeFile::new(Path::new(entry[1])));
    }
    i += 1;
  }
  
  let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
  if env::var("SKIP_WATCH").is_ok_and(is_truthy) {
    if !env::var("SILENT").is_ok_and(is_truthy) {
      println!("Serving {}/* at http://localhost:{}", base_dir.display(), port);
    }
    
    axum::serve(
      listener,
      r.layer(middleware::from_fn(insert_serve_headers)),
    ).await.unwrap();
  } else {
    if !env::var("SILENT").is_ok_and(is_truthy) {
      println!("Watching+Serving {}/* at http://localhost:{}", base_dir.display(), port);
    }
    
    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();
    
    let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
    watcher.watch(base_dir.as_path(), notify::RecursiveMode::Recursive)?;
    
    axum::serve(
      listener,
      r.layer(middleware::from_fn(insert_watch_headers)).layer(livereload),
    ).await.unwrap();
  }
  Ok(())
}
