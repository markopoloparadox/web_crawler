<h1 align="center">Async Web Crawler With Rust 🦀</h1>
<div align="center">
  <h4>
    <a href="#install">
      Install
    </a>
    <span> | </span>
    <a href="#usage">
      Usage
    </a>
    <span> | </span>
    <a href="#final-comments">
      Final Comments
    </a>
  </h4>
</div>
<br>

This repository is meant be used as a starting point for building more complex Web Crawler.

This repo contains the following features:

- Basic HTTP Web Server and API implementation using the [Tide](https://github.com/http-rs/tide) library. It contains examples for `post` and `get` http methods.
- Simple Spider module which has all the necessary tools to do the crawling.
- Mechanism to archive(download) crawled websites

# Install
[Nightly rust](https://doc.rust-lang.org/edition-guide/rust-2018/rustup-for-managing-rust-versions.html) is required in order to build and run this project. All the necessary dependencies will be installed once `cargo check` or `cargo run` are called.

For general information about how to install Rust look [here](https://www.rust-lang.org/tools/install).

# Usage

## Server
The server is started by executing the following command inside the root project folder:
```
$ cargo run
```

This should hopefully run the HTTP Web Server and the following message
should be visible inside the terminal:
```
tide::log Logger started
    level Info
tide::server Server listening on http://127.0.0.1:8080
```

## Client
The server exposes three API that are available to be called.
### HTTP Post /spider
This API takes as an input a JSON object that contains a domain address and several optional parameters. On success, it outputs a JSON object which contains the id the of the crawled domain. That ID can be later used to get the list or count of links that were crawled.

### Input
```
{
    "address": "https://www.google.com" # This parameter is required.
    "max_depth": 2                      # This parameter is optional.
                                          This refers to how far down into a
                                          website's page hierarchy the spider
                                          will crawl. If left unset, no limit
                                          will be applied.
    "max_pages": 2                      # This parameter is optional.
                                          This refers how many pages will the
                                          spider crawl before it stops. If
                                          left unset, no limit will be applied.
    "robots_txt": false                 # This parameter is optional.
                                          If enabled, the spider will slow
                                          the speed of crawling and/or ignore
                                          certain subdomain. This
                                          parameter is currently unused.
    "archive_pages": false              # This parameter is optional.
                                          If enabled, the spider will archive
                                          (download) the crawled web pages
                                          If left unset, the default value is
                                          used which is false.
}
```

### Output
```
{
    "id": "ABCDEFGHT"   # MDA5 hash that is used as an ID.
}
```

### Example
If the server is running, run the following command inside a new terminal:
```
$ curl localhost:8080/spider -d '{ "address": "http://www.zadruga-podolski.hr" }'
{ "id":"e0436759bf33e12eb53ae0b97f790991" }
```

### HTTP GET /spider/:id/list
This API return the list of crawled web pages for a specific domain. The ID is retrieved by calling `post /spider`.

### Full Example
If the server is running, run the following command inside a new terminal:
```
$ curl localhost:8080/spider -d '{ "address": "http://www.zadruga-podolski.hr" }'
{ "id":"e0436759bf33e12eb53ae0b97f790991" }

$ curl localhost:8080/spider/e0436759bf33e12eb53ae0b97f790991/list
["http://www.zadruga-podolski.hr/kontakt.html","http://www.zadruga-podolski.hr/muškat-žuti.html","http://www.zadruga-podolski.hr/diplome-i-priznanja.html","http://www.zadruga-podolski.hr/chardonnay.html","http://www.zadruga-podolski.hr/o-nama.html","http://www.zadruga-podolski.hr/index.html","http://www.zadruga-podolski.hr/graševina-ledeno-vino.html","http://www.zadruga-podolski.hr/tradicija-i-običaji.html","http://www.zadruga-podolski.hr/križevci.html","http://www.zadruga-podolski.hr/pinot-sivi.html","http://www.zadruga-podolski.hr/pinot-bijeli.html","http://www.zadruga-podolski.hr","http://www.zadruga-podolski.hr/graševina.html"]
```

### HTTP GET /spider/:id/count
This API return the count of crawled web pages for a specific domain. The ID is retrieved by calling `post /spider`.

### Full Example
If the server is running, run the following command inside a new terminal:
```
$ curl localhost:8080/spider -d '{ "address": "http://www.zadruga-podolski.hr" }'
{ "id":"e0436759bf33e12eb53ae0b97f790991" }

$ curl localhost:8080/spider/e0436759bf33e12eb53ae0b97f790991/count
{ "count": 13 }
```

# Final comments
In order to keep this project simple and small, certain features were intentionally unimplemented like checking for the "robots.txt" file or checking the header for the `<base>` tag. Fell free to implement those features by yourself as a kind of exercise. Also, this project heavily relies on async code so anyone who is a async-first-timer should definitely check out this two videos [video1](https://www.youtube.com/watch?v=lJ3NC-R3gSI) [video2](https://www.youtube.com/watch?v=NNwK5ZPAJCk)