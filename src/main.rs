use bat::PrettyPrinter;
use clap::Parser;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{
    header::{HeaderMap, ACCEPT, USER_AGENT},
    Client,
};
use scraper::{ElementRef, Html, Selector};
use std::{cmp::min, fmt::Write as _, io::Write as _};
use url::Url;

/// Simple tool to download and parse HTML
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// which page to download
    url: Option<String>,

    /// select html from the downloaded age
    selector: Option<String>,

    /// select a certain attribute
    #[clap(short, long)]
    attribute: Option<String>,

    /// do not print progress or warnings
    #[clap(short, long)]
    quiet: bool,

    /// pretend to be Mozilla, like everyone else
    #[clap(short, long)]
    mozilla: bool,

    /// print headers
    #[clap(long, env = "HEADERS")]
    headers: bool,

    /// print count nodes only
    #[clap(long, short = 'n')]
    count: Option<usize>,
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

async fn download(
    client: &Client,
    url: &Url,
    Args {
        headers: print_headers,
        mozilla,

        quiet,
        ..
    }: &Args,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();

    if *mozilla {
        headers.insert(
            ACCEPT,
            "text/html,application/xhtml+xml,application/xml".parse()?,
        );
        headers.insert(USER_AGENT, "Mozilla/5.0".parse()?);
    }

    if_log(|| eprintln!("request headers {headers:#?}"));

    // Reqwest setup
    let res = client
        .get(url.as_str())
        .headers(headers)
        .send()
        .await
        .map_err(|_| format!("Failed to GET from '{}'", &url))?;

    if *print_headers {
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
        if_log(|| eprintln!("no content-length header for '{}'", &url));

        Ok(res.text().await?)
    }
}

fn parse_url(input: &str) -> Result<Url, url::ParseError> {
    Url::parse(input).or_else(|e| match e {
        url::ParseError::RelativeUrlWithoutBase => Url::parse(&format!("https://{input}")),
        _ => Err(e),
    })
}

fn take_nodes<'a>(
    document: &'a Html,
    selector: &'a Selector,
    count: Option<usize>,
) -> Box<dyn Iterator<Item = ElementRef<'a>> + 'a> {
    if let Some(count) = count {
        Box::new(document.select(&selector).take(count))
    } else {
        Box::new(document.select(&selector))
    }
}

fn if_log(then: impl Fn()) {
    static PASSWORD: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *PASSWORD.get_or_init(|| std::env::var("RUST_LOG").is_ok()) {
        then()
    }
}

async fn the_main() -> Result<(), Box<dyn std::error::Error>> {
    let args @ Args {
        url,
        selector,
        attribute,
        count,
        ..
    } = &Args::parse();

    if let Some(url) = url {
        let url = parse_url(&url)?;
        let client = reqwest::Client::new();

        let content = if let Some(selector) = selector {
            let selector = Selector::parse(&selector).map_err(|_| "Invalid selector")?;
            let body = download(&client, &url, &args).await?;

            let document = Html::parse_document(&body);
            document.select(&selector);

            let mut content = String::new();
            for node in take_nodes(&document, &selector, *count) {
                if let Some(attribute) = attribute.as_ref().and_then(|a| node.value().attr(a)) {
                    writeln!(&mut content, "{}", attribute.to_owned())?;
                } else {
                    writeln!(&mut content, "{}", node.inner_html())?;
                }
            }
            content
        } else {
            download(&client, &url, &args).await?
        };

        PrettyPrinter::new()
            .input_from_bytes(content.as_bytes())
            .theme("1337")
            .language("html")
            .print()?;
        println!();
    } else {
        if_log(|| eprintln!("need to give me a URL"))
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
