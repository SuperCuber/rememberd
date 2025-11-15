#[macro_use]
extern crate serde_json;

use std::time::Instant;

use actix_web::{
    web::{self, Bytes},
    App, HttpResponse, HttpServer, Responder,
};
use futures::{stream, StreamExt, TryStreamExt};
use handlebars::Handlebars;
use lazy_static::lazy_static;
use tokio::sync::{Mutex, MutexGuard};

lazy_static! {
    static ref TEXT: Mutex<String> = Mutex::new(String::new());
    static ref LAST_UPDATED: Mutex<Instant> = Mutex::new(Instant::now());
    static ref UPLOADED_FILE: Mutex<(String, Vec<u8>)> = Mutex::new((String::new(), Vec::new()));
}

const TEXT_TIMEOUT: u64 = 60 * 60;

async fn read(hb: web::Data<Handlebars<'_>>) -> impl Responder {
    let mut text = TEXT.lock().await.clone();
    let mut file = UPLOADED_FILE.lock().await.clone();
    if LAST_UPDATED.lock().await.elapsed().as_secs() > TEXT_TIMEOUT {
        text = "".into();
        file = (String::new(), Vec::new());
    }

    let data = json!({ "text": text, "filename": file.0 });
    let body = hb.render("index", &data).unwrap();

    HttpResponse::Ok().content_type("text/html").body(body)
}

async fn update(body: String) -> impl Responder {
    let mut text: MutexGuard<String> = TEXT.lock().await;
    *text = body;

    let mut last_updated: MutexGuard<Instant> = LAST_UPDATED.lock().await;
    *last_updated = Instant::now();

    // Return something
    "success"
}

async fn upload(mut payload: actix_multipart::Multipart) -> Result<HttpResponse, actix_web::Error> {
    // iterate over multipart stream
    let mut file = UPLOADED_FILE.lock().await;
    file.1.clear();
    let mut last_updated = LAST_UPDATED.lock().await;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();
        file.0 = filename.into();

        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            file.1.extend(data);
        }
    }
    *last_updated = Instant::now();
    Ok(HttpResponse::Ok().into())
}

async fn download() -> impl Responder {
    let file = UPLOADED_FILE.lock().await;

    HttpResponse::Ok().streaming(stream::iter(vec![Result::<_, actix_web::Error>::Ok(
        Bytes::copy_from_slice(&file.1),
    )]))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Initialize handlebars' template repository
    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./templates")
        .unwrap();
    let handlebars_ref = web::Data::new(handlebars);

    HttpServer::new(move || {
        App::new()
            .app_data(handlebars_ref.clone())
            .service(
                web::resource("/")
                    .route(web::get().to(read))
                    .route(web::put().to(update))
                    .route(web::post().to(upload)),
            )
            .service(web::resource("/download/{name}").route(web::get().to(download)))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
