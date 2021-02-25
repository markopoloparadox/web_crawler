<h1 align="center">Async Web Crawler With Rust 🦀</h1>
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

Input:
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

Output:
```
{
    "id": "ABCDEFGHT"   # MDA5 hash that is used as an ID.
}
```