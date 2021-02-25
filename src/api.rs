use crate::spider::{Spider, SpiderOptions};
use crate::State;
use serde::{Deserialize, Serialize};
use tide::{Body, Request, Response};

#[derive(Deserialize, Serialize)]
struct PostCrawlAnswer {
    pub id: String,
}

pub async fn post_spider(mut req: Request<State>) -> tide::Result {
    let input: Input = req.body_json().await?;
    let hash = input.generate_hash();

    // Create successful response object
    let body = PostCrawlAnswer { id: hash.clone() };
    let mut successful_response = Response::new(201);
    successful_response.set_body(Body::from_json(&body)?);

    // Check if we have already crawled the requested domain
    {
        let database = req.state().database.lock().await;
        if database.domain_links.contains_key(&hash) {
            return Ok(successful_response);
        }
    }

    let robots_txt = input.robots_txt.unwrap_or(false);
    let archive_pages = input.archive_pages.unwrap_or(false);

    let options = SpiderOptions::new(input.max_depth, input.max_pages, robots_txt, archive_pages);
    let database = req.state().database.clone();
    let value = Spider::run(input.address, options, database).await;

    let value = match value {
        Some(x) => x,
        None => return Ok("error".to_owned().into()),
    };

    {
        let mut database = req.state().database.lock().await;
        database.domain_links.insert(hash.clone(), value);
    }

    // Return
    Ok(successful_response)
}

pub async fn get_spider_list(req: Request<State>) -> tide::Result {
    let database = req.state().database.lock().await;
    let id = req.param("id")?;

    if let Some(list) = database.domain_links.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(list)?);
        return Ok(res);
    }

    let mut res = Response::new(400);
    res.set_body(Body::from_json(&"Unknown id".to_owned())?);
    return Ok(res);
}

pub async fn get_spider_count(req: Request<State>) -> tide::Result {
    let database = req.state().database.lock().await;
    let id = req.param("id")?;

    if let Some(list) = database.domain_links.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&list.len())?);
        return Ok(res);
    }

    let mut res = Response::new(400);
    res.set_body(Body::from_json(&"Unknown id".to_owned())?);
    return Ok(res);
}

#[derive(Deserialize, Serialize, Debug)]
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
