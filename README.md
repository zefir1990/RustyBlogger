# RustyBlogger

Multilingual blog engine written in Rust. It imports WordPress-style `.txt` posts into a [`redb`](https://github.com/cberner/redb) embedded database, renders them as full HTML pages, and serves them with [`axum`](https://github.com/tokio-rs/axum).

## Features

- Fast embedded storage with `redb`
- Multilingual posts with language switching and fallback
- Post dates taken from each file's first git commit, newest first
- Full posts on the home page with pagination
- Configurable posts per page stored in the database
- Category pages
- Media serving from a local directory
- JSON API
- Markdown-like conveniences: inline code, bullet and numbered lists, links, images
- Localization blocks: `<RUONLY>...</RUONLY>` and `<NOTRU>...</NOTRU>`

## Build

Requires Rust 1.75 or newer.

```bash
cargo build --release
```

The binary is `target/release/rustyblogger`.

## Quick Start

```bash
target/release/rustyblogger import \
  --posts-directory /Users/zefir1990/Sources/blog/src \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --database blog.redb

target/release/rustyblogger serve \
  --database blog.redb \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --bind-address 127.0.0.1:8080
```

Open http://127.0.0.1:8080. The site title is read from `BlogSettings.title` in `redb`; `--site-title` on `serve` can override it.

## Commands

### Validate

```bash
target/release/rustyblogger validate \
  --posts-directory /Users/zefir1990/Sources/blog/src \
  --media-directory /Users/zefir1990/Sources/blog/media
```

Warnings are reported but do not fail validation. Use `--strict` to fail on warnings.

### Import

```bash
target/release/rustyblogger import \
  --posts-directory /Users/zefir1990/Sources/blog/src \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --database blog.redb \
  --posts-per-page 10
```

Import aborts on syntax errors. Warnings are tolerated unless `--strict` is used.

Options:

- `--default-language` — source language, default `ru`
- `--languages` — comma-separated language list for the switcher, default `ru,en,zh,de,ja,fr,es,pt,hi`
- `--posts-per-page` — page size stored in `redb`, default `10`
- `--site-title` — site title stored in `BlogSettings`, default `Zefir1990`

### Serve

```bash
target/release/rustyblogger serve \
  --database blog.redb \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --bind-address 127.0.0.1:8080
```

## Routes

| Route | Description |
| --- | --- |
| `/` | Paginated home page with full posts |
| `/?page=N` | Select page |
| `/category/{category}?page=N` | Posts in a category |
| `/posts/{slug}?lang={code}` | Single post |
| `/language/{code}?next=/path` | Set language cookie and redirect |
| `/api/posts` | Posts as JSON |
| `/api/posts/{slug}` | Single post as JSON |
| `/api/languages` | Available languages |
| `/api/languages/{language}/posts` | Posts in one language |
| `/media/*` | Media files |
| `/health`, `/api/health` | Health check |

Language selection order: `?lang=`, language cookie, `Accept-Language`, then the database default language.

## Post Format

Posts are UTF-8 `.txt` files with exactly five metadata lines followed by the body.

```text
Format: Fall24-October10
Language: ru
Title: Example post
Slug: example-post
Categories: software
Post body starts here.
```

Rules:

- `Format` must be `Fall24-October10`
- Source language is Russian (`ru`)
- `Slug` uses lowercase latin letters, digits, and hyphens
- Categories are expected to be `software` or `notes`
- Code blocks are plain `<pre><code>...</code></pre>`
- Standalone links must be on their own line
- Image lines contain a filename such as `image.jpg`, or `https://example.com/page/|image.jpg`
- `[DO NOT PROCESS LINE]` strips the directive and leaves the rest untouched
- The creation date is the file's first git commit date; files without git history fall back to file modification time

## Translations

Use the same slug in another file:

```text
Format: Fall24-October10
Language: en
Title: Example post
Slug: example-post
Categories: software
Translated body starts here.
```

Save it as `example-post.en.txt`. Missing translations fall back to the slug's default language. The language selector still lists all configured languages so users can switch and see the fallback notice.

## Database

`redb` tables:

- `posts` — post variants keyed by slug and language
- `slugs` — available languages per slug
- `language_index` — summaries grouped by language
- `languages` — language metadata
- `metadata` — default language and import timestamp
- `settings` — blog settings, including `posts_per_page`

Generated `*.redb` files are gitignored.

## Development

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
```

See `AGENTS.md` for repository-specific guidelines.
