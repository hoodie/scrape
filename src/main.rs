use clap::Parser;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use scraper::{Html, Selector};
use std::{cmp::min, io::Write};
use url::Url;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// which page to download
    url: Option<String>,

    /// select html from the downloaded age
    selector: Option<String>,

    #[clap(short, long)]
    attribute: Option<String>,

    #[clap(short, long)]
    quiet: bool,

    #[clap(long, env = "HEADERS")]
    headers: bool,
}

fn progress_bar(total_size: u64, url: &str) -> ProgressBar {
    let progress_bar = ProgressBar::new(total_size);

    progress_bar.set_style(
            ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("█>-"));

    progress_bar.set_message(format!("Downloading {}", url));
    progress_bar
}

pub async fn download(
    client: &Client,
    url: &Url,
    print_headers: bool,
    quiet: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    // Reqwest setup
    let res = client
        .get(url.as_str())
        .send()
        .await
        .map_err(|_| format!("Failed to GET from '{}'", &url))?;

    if print_headers {
        eprintln!("{:#?}", res.headers());
    }

    if let Some(total_size) = res.content_length() {
        let progress_bar = (!quiet).then(|| progress_bar(total_size, url.as_str()));

        // download chunks
        let mut buffer = Vec::with_capacity(total_size as usize);
        let mut downloaded: u64 = 0;
        let mut stream = res.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|_| "Error while downloading file")?;
            buffer
                .write_all(&chunk)
                .map_err(|_| "Error while writing to file")?;

            let new = min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;

            if let Some(ref progress_bar) = progress_bar {
                progress_bar.set_position(new);
            }
        }

        if let Some(progress_bar) = progress_bar {
            progress_bar.finish_and_clear();
        }

        Ok(String::from_utf8(buffer)?)
    } else {
        if !quiet {
            eprintln!("no content-length header for '{}'", &url);
        }

        Ok(res.text().await?)
    }
}

fn parse_url(input: &str) -> Result<Url, url::ParseError> {
    let parsed = Url::parse(input).or_else(|e| match e {
        url::ParseError::RelativeUrlWithoutBase => Url::parse(&format!("https://{input}")),
        _ => Err(e),
    });
    parsed
}

async fn the_main() -> Result<(), Box<dyn std::error::Error>> {
    let Args {
        url,
        selector,
        attribute,
        quiet,
        headers,
    } = Args::parse();
    if let Some(url) = url {
        let url = parse_url(&url)?;
        let client = reqwest::Client::new();
        let body = download(&client, &url, headers, quiet).await?;

        if let Some(selector) = selector {
            let selector = Selector::parse(&selector).unwrap();

            let document = Html::parse_document(&body);
            document.select(&selector);

            for node in document.select(&selector) {
                if let Some(attribute) = attribute.as_ref().and_then(|a| node.value().attr(a)) {
                    println!("{}", attribute);
                } else {
                    println!("{}", node.inner_html().trim());
                }
            }
        } else {
            println!("{}", body);
        }
    } else {
        eprintln!("need to give me a URL");
    }
    Ok(())
}

#[cfg(feature = "multi")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    the_main().await
}

#[cfg(feature = "single")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            the_main().await.unwrap();
        });
    Ok(())
}
