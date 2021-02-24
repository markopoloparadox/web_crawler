#![feature(proc_macro_hygiene, decl_macro)]

mod api;
mod spider;
mod state;

/*  Uncomment to enable CORS
    use http_types::headers::HeaderValue;
    use tide::security::{CorsMiddleware, Origin};
*/

use api::{get_spider_count, get_spider_list, post_spider};
use spider::Spider;
use state::State;

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();

    let state = State::new();
    let mut app = tide::with_state(state);

    /*  Uncomment to enable CORS
        let rules = CorsMiddleware::new()
            .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
            .allow_origin(Origin::from("*"))
            .allow_credentials(false);
        app.with(rules);
    */
    app.at("/spider").post(post_spider);
    app.at("/spider/:id/list").get(get_spider_list);
    app.at("/spider/:id/count").get(get_spider_count);

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
