#[macro_use]
extern crate serde_json;

use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use lazy_static::lazy_static;
use handlebars::Handlebars;

lazy_static! {
    static ref TEXT: Mutex<String> = Mutex::new(String::new());
    static ref LAST_UPDATED: Mutex<Instant> = Mutex::new(Instant::now());
}

const TEXT_TIMEOUT: u64 = 1 * 60 * 60;

async fn read(hb: web::Data<Handlebars<'_>>) -> impl Responder {
    let mut text: String = TEXT.lock().expect("lock mutex").clone();
    // TODO: add some html around this that will AJAX some PUTs when changing
    // TODO: set up templating for the text into the html (make sure you escape it properly, what if I want to paste some html in?)
    if LAST_UPDATED.lock().expect("lock mutex").elapsed().as_secs() > TEXT_TIMEOUT {
        text = "".into();
    }

    let data = json!({ "text": text });
    let body = hb.render("index", &data).unwrap();

    HttpResponse::Ok().body(body)
}

async fn update(body: String) -> impl Responder {
    let mut text: MutexGuard<String> = TEXT.lock().expect("lock mutex");
    *text = body;

    let mut last_updated: MutexGuard<Instant> = LAST_UPDATED.lock().expect("lock mutex");
    *last_updated = Instant::now();

    // Return something to the AJAX
    "success"
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
        App::new().app_data(handlebars_ref.clone()).service(
            web::resource("/")
                .route(web::get().to(read))
                .route(web::put().to(update)),
        )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
