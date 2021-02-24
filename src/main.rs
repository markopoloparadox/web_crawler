#![feature(proc_macro_hygiene, decl_macro)]

mod api;
mod state;
use api::{count_unique_url, get_crawl, list_unique_url};
use state::State;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let state = State::new();

    let mut app = tide::with_state(state);
    app.at("/crawl").get(get_crawl);
    app.at("/crawl/:id/list").get(list_unique_url);
    app.at("/crawl/:id/count").get(count_unique_url);

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
