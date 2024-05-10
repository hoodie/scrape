<div align="center">

# scrape

</div>

```bash
Simple tool to download and parse HTML

Usage: scrape [OPTIONS] <URL> [SELECTOR]

Arguments:
  <URL>       which page to download
  [SELECTOR]  select html from the downloaded page (css selector)

Options:
  -a, --attribute <ATTRIBUTE>  select a certain attribute
  -q, --quiet                  do not print progress or warnings
  -m, --mozilla                pretend to be Mozilla, like everyone else
      --headers                print headers [env: HEADERS=]
  -n, --count <COUNT>          print count nodes only
  -h, --help                   Print help
  -V, --version                Print version
```
