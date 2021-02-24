use serde::{Deserialize, Serialize};
use tide::{Body, Request, Response};

use crate::{Spider, State};

#[derive(Deserialize, Serialize, Debug)]
struct Domain {
    address: String,
    max_depth: Option<usize>,
    max_pages: Option<usize>,
}

#[derive(Deserialize, Serialize)]
struct PostCrawlAnswer {
    pub id: String,
}

pub async fn post_spider(mut req: Request<State>) -> tide::Result {
    let domain: Domain = req.body_json().await?;
    println!("Domain name: {:#?}", domain);

    let key = format!("{:?}", domain);
    let key = format!("{:x}", md5::compute(key));

    let value = Spider::run(&domain.address, domain.max_depth, domain.max_pages).await;
    let value = match value {
        Some(x) => x,
        None => return Ok("error".to_owned().into()),
    };

    {
        let mut database = req.state().database.lock().await;
        database.domain_links.insert(key.clone(), value);
    }

    // Return
    let body = PostCrawlAnswer { id: key };
    let mut res = Response::new(201);
    res.set_body(Body::from_json(&body)?);
    return Ok(res);
}

pub async fn get_spider_list(req: Request<State>) -> tide::Result {
    let database = req.state().database.lock().await;
    let id = req.param("id")?;

    if let Some(list) = database.domain_links.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(list)?);
        return Ok(res);
    }

    Ok("Failed".into())
}

pub async fn get_spider_count(req: Request<State>) -> tide::Result {
    let database = req.state().database.lock().await;
    let id = req.param("id")?;

    if let Some(list) = database.domain_links.get(id) {
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&list.len())?);
        return Ok(res);
    }

    Ok("Failed".into())
}
