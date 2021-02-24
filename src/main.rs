#![feature(proc_macro_hygiene, decl_macro)]

mod api;
mod state;

/*  Uncomment to enable CORS
    use http_types::headers::HeaderValue;
    use tide::security::{CorsMiddleware, Origin};
*/

use api::{get_crawled_count, get_crawled_list, post_crawl};
use state::State;

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();

    let state = State::new();

    /*  Uncomment to enable CORS
        let rules = CorsMiddleware::new()
            .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
            .allow_origin(Origin::from("*"))
            .allow_credentials(false);
    */

    let mut app = tide::with_state(state);

    /*  Uncomment to enable CORS
        app.with(rules);
    */
    app.at("/crawl").post(post_crawl);
    app.at("/crawl/:id/list").get(get_crawled_list);
    app.at("/crawl/:id/count").get(get_crawled_count);

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
