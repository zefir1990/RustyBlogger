# RustyBlogger

Multilingual blog engine written in Rust. It reads posts from a [`redb`](https://github.com/cberner/redb) embedded database, renders them as full HTML pages, and serves them with [`axum`](https://github.com/tokio-rs/axum).

Live example: [https://zefir1990.space](https://zefir1990.space)

## Features

- Fast embedded storage with `redb`
- Multilingual posts with language switching and fallback
- Posts sorted newest first
- Localized GDPR cookie notice in all nine interface languages
- Full posts on the home page with pagination
- Configurable posts per page stored in the database
- Category pages
- Media serving from a local directory
- JSON API

## Build

Requires Rust 1.75 or newer.

```bash
cargo build --release
```

The binary is `target/release/rustyblogger`.

## Quick Start

```bash
target/release/rustyblogger serve \
  --database blog.redb \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --bind-address 127.0.0.1:8080
```

Open http://127.0.0.1:8080. The site title is read from `BlogSettings.title` in `redb`; `--site-title` on `serve` can override it.

## Commands

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

## Database

`redb` tables:

- `posts` — post variants keyed by slug and language
- `slugs` — available languages per slug
- `language_index` — summaries grouped by language
- `languages` — language metadata
- `metadata` — default language
- `settings` — blog settings, including `posts_per_page`

Generated `*.redb` files are gitignored.

## Development

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
```

See `AGENTS.md` for repository-specific guidelines.
