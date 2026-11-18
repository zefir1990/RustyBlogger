# RustyBlogger

Multilingual blog engine written in Rust. It reads posts from a `redb` embedded database, renders them as full HTML pages, and serves them through `axum`.

Media lives in `/Users/zefir1990/Sources/blog/media`.

## Commands

Run all commands from the repository root.

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo build --release
```

The release binary is `target/release/rustyblogger`.

```bash
target/release/rustyblogger serve \
  --database blog.redb \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --bind-address 127.0.0.1:8080
```

## Architecture

| File | Responsibility |
| --- | --- |
| `src/main.rs` | CLI dispatch and command orchestration |
| `src/cli.rs` | Clap command definitions |
| `src/model.rs` | Domain models and settings |
| `src/render.rs` | HTML escaping helpers |
| `src/storage.rs` | `BlogRepository` trait and `redb` implementation |
| `src/web.rs` | Axum routes, language resolution, pagination |
| `src/templates.rs` | HTML layout, post pages, home page, pagination, CSS |

Keep storage behind `BlogRepository`. Rendering must stay independent of HTTP and database code. Prefer small focused functions and explicit names.

## Database

`redb` tables:

- `posts` — variant key `{slug}\u{1f}{language}` to serialized `PostRecord`
- `slugs` — slug to `SlugRecord` with available languages and default language
- `language_index` — `{language}\u{1f}{slug}` to serialized `PostSummary`
- `languages` — language code to `LanguageRecord`
- `metadata` — default language
- `settings` — key `blog_settings` to serialized `BlogSettings`

`BlogSettings` currently contains `posts_per_page` (default `10`) and `title` (default `Zefir1990`). The home page reads pagination settings from the database, and the server reads the site title from settings unless `--site-title` is passed.

Generated `*.redb` files are gitignored.

## Routes

- `/` and `/?page=N` — paginated home page with full posts
- `/category/{category}?page=N`
- `/posts/{slug}?lang={code}`
- `/language/{code}?next=/path` — sets the language cookie and redirects
- `/api/posts`, `/api/posts/{slug}`, `/api/languages`
- `/api/languages/{language}/posts`
- `/media/*` — media files
- `/health` and `/api/health`

Language resolution order: `?lang=`, language cookie, `Accept-Language`, then the database default language.

## Verification

Before finishing a change:

1. Run `cargo fmt --check`, `cargo check`, and `cargo clippy -- -D warnings`.
2. Smoke-test `/`, a post route, pagination, language switching, the API, and `/media`.
3. Do not modify `/Users/zefir1990/Sources/blog` unless explicitly asked.
4. Do not commit unless explicitly asked.

## Code Style

Follow the global instructions in `~/.config/opencode/AGENTS.md`:

- No code comments; make the code self-documenting.
- Use full descriptive names.
- Apply SOLID, single responsibility, decomposition, and DRY.
- Prefer strict typing and explicit domain types.
- Do not add unnecessary blank lines.
