# git-reel

Git Reel is a local-first GitHub repository discovery app.

## Development

Install frontend dependencies:

```bash
npm install
```

Run the local API:

```bash
cargo run --manifest-path server/Cargo.toml
```

Run the web app:

```bash
npm run dev:web
```

Run tests:

```bash
npm test
```

## End-to-end tests

Install the browser once:

```bash
npx playwright install chromium
```

Run the local flow:

```bash
npm run test:e2e
```
