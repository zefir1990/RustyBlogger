# RustyBlogger

Multilingual blog engine written in Rust. It imports WordPress-style `.txt` posts into a `redb` embedded database, renders them as full HTML pages, and serves them through `axum`.

Source posts live in `/Users/zefir1990/Sources/blog/src`; media lives in `/Users/zefir1990/Sources/blog/media`.

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
target/release/rustyblogger validate \
  --posts-directory /Users/zefir1990/Sources/blog/src \
  --media-directory /Users/zefir1990/Sources/blog/media

target/release/rustyblogger import \
  --posts-directory /Users/zefir1990/Sources/blog/src \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --database blog.redb

target/release/rustyblogger serve \
  --database blog.redb \
  --media-directory /Users/zefir1990/Sources/blog/media \
  --bind-address 127.0.0.1:8080

tools/import-localizations --slug strata-local-ai --database blog.redb
```

`import-localizations` requires the server to be stopped because `redb` is single-writer. `validate --strict` fails on warnings. Import aborts on errors; warnings are tolerated unless `--strict` is passed. Import also accepts `--posts-per-page`, `--site-title`, `--languages`, and `--default-language`.

## Architecture

| File | Responsibility |
| --- | --- |
| `src/main.rs` | CLI dispatch and command orchestration |
| `src/cli.rs` | Clap command definitions |
| `src/model.rs` | Domain models, settings, validation issues |
| `src/parser.rs` | Post metadata parsing and validation |
| `src/render.rs` | Blog body rendering, localization tags, media and code blocks |
| `src/importer.rs` | Builds a `BlogSnapshot` from post files |
| `src/storage.rs` | `BlogRepository` trait and `redb` implementation |
| `src/web.rs` | Axum routes, language resolution, pagination |
| `src/templates.rs` | HTML layout, post pages, home page, pagination, CSS |
| `src/wordpress.rs` | WordPress localization import by post slug |
| `tools/import-localizations` | Wrapper for `import-wordpress` |

Keep storage behind `BlogRepository`. Parsing and rendering must stay independent of HTTP and database code. Prefer small focused functions and explicit names.

## Database

`redb` tables:

- `posts` — variant key `{slug}\u{1f}{language}` to serialized `PostRecord`
- `slugs` — slug to `SlugRecord` with available languages and default language
- `language_index` — `{language}\u{1f}{slug}` to serialized `PostSummary`
- `languages` — language code to `LanguageRecord`
- `metadata` — default language and import timestamp
- `settings` — key `blog_settings` to serialized `BlogSettings`

`BlogSettings` currently contains `posts_per_page` (default `10`) and `title` (default `Zefir1990`). The home page reads pagination settings from the database, and the server reads the site title from settings unless `--site-title` is passed.

Generated `*.redb` files are gitignored.

## Blog Post Format

Posts are UTF-8 `.txt` files with exactly five metadata lines followed by the body.

```text
Format: Fall24-October10
Language: ru
Title: <title>
Slug: <english-slug>
Categories: software|notes
<post body>
```

Rules:

- `Format` must be `Fall24-October10`.
- Source language is Russian (`ru`).
- Slug uses lowercase latin letters, digits, and hyphens.
- Categories in the source repository are expected to be `software` or `notes`.
- Code blocks are plain `<pre><code>...</code></pre>` without a language specification.
- Standalone links must be on their own line.
- An image line contains a filename such as `image.jpg`, or `https://example.com/page/|image.jpg`.
- `[DO NOT PROCESS LINE]` strips the directive and leaves the remainder untouched.
- `<RUONLY>...</RUONLY>` renders only for Russian; `<NOTRU>...</NOTRU>` renders only for non-Russian languages.
- Post dates come from the file's first git commit date (creation date); files without git history fall back to file modification time. Posts are sorted newest first.

Translations use the same `Slug` in another file, for example `ghost-contacts.en.txt` with `Language: en`. The language selector includes the default language list even before translations exist; missing translations fall back to the slug's default language.

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
2. Run `validate` against the blog source directory.
3. Import into a temporary database and smoke-test `/`, a post route, pagination, language switching, the API, and `/media`.
4. Do not modify `/Users/zefir1990/Sources/blog` unless explicitly asked.
5. Do not commit unless explicitly asked.

## Code Style

Follow the global instructions in `~/.config/opencode/AGENTS.md`:

- No code comments; make the code self-documenting.
- Use full descriptive names.
- Apply SOLID, single responsibility, decomposition, and DRY.
- Prefer strict typing and explicit domain types.
- Do not add unnecessary blank lines.
