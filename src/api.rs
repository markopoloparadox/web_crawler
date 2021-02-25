use crate::spider::{Spider, SpiderOptions};
use crate::State;
use serde::{Deserialize, Serialize};
use tide::{Body, Request, Response};

pub async fn post_spider(mut req: Request<State>) -> tide::Result {
    let input: Input = req.body_json().await?;
    let hash = input.generate_hash();

    // Create successful response object
    let body = Output { id: hash.clone() };
    let mut successful_response = Response::new(201);
    successful_response.set_body(Body::from_json(&body)?);

    // If we have already crawled the requested domain return the id
    {
        let database = req.state().database.lock().await;
        if database.domain_links.contains_key(&hash) {
            return Ok(successful_response);
        }
    }

    // Prepare data for crawling
    let robots_txt = input.robots_txt.unwrap_or(false);
    let archive_pages = input.archive_pages.unwrap_or(false);
    let options = SpiderOptions::new(input.max_depth, input.max_pages, robots_txt, archive_pages);
    let database = req.state().database.clone();

    // Crawl
    let list = Spider::run(input.address, options, database).await;

    // Save found list
    {
        let mut database = req.state().database.lock().await;
        database.domain_links.insert(hash.clone(), list);
    }

    // Return
    Ok(successful_response)
}

pub async fn get_spider_list(req: Request<State>) -> tide::Result {
    // Retrieve id from request
    let id = req.param("id");
    if id.is_err() {
        let mut response = Response::new(400);
        response.set_body(Body::from_json(&"Missing id param".to_owned())?);
        return Ok(response);
    }
    let id = id.unwrap();

    // Retrieve list
    let database = req.state().database.lock().await;
    let (status_code, body) = match database.domain_links.get(id) {
        Some(x) => (200, Body::from_json(x)?),
        None => (400, Body::from_json(&"Unknown id".to_owned())?),
    };

    let mut response = Response::new(status_code);
    response.set_body(body);
    Ok(response)
}

pub async fn get_spider_count(req: Request<State>) -> tide::Result {
    // Retrieve id from request
    let id = req.param("id");
    if id.is_err() {
        let mut response = Response::new(400);
        response.set_body(Body::from_json(&"Missing id param".to_owned())?);
        return Ok(response);
    }
    let id = id.unwrap();

    // Retrieve count
    let database = req.state().database.lock().await;
    let (status_code, body) = match database.domain_links.get(id) {
        Some(x) => (200, Body::from_json(&x.len().to_string())?),
        None => (400, Body::from_json(&"Unknown id".to_owned())?),
    };

    let mut response = Response::new(status_code);
    response.set_body(body);
    Ok(response)
}

#[derive(Deserialize, Serialize)]
struct Input {
    address: String,
    max_depth: Option<usize>,
    max_pages: Option<usize>,
    robots_txt: Option<bool>,
    archive_pages: Option<bool>,
}

impl Input {
    pub fn generate_hash(&self) -> String {
        let hash = format!("{:?}{:?}{:?}", self.address, self.max_depth, self.max_pages);
        format!("{:x}", md5::compute(hash))
    }
}

#[derive(Deserialize, Serialize)]
struct Output {
    pub id: String,
}
