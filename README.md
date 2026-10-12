# TableTalk

> Voice-driven restaurant intelligence powered by the Model Context Protocol (MCP).

**Status**: in development

TableTalk is a restaurant Model Context Protocol (MCP) server that empowers voice assistants to search menus, check table availability, book reservations, and place orders. A React web frontend simulates the voice assistant experience for real-time demos and evaluation.

---

## Hackathon

- **Competition**: Amazon Developer Hackathon
- **Track**: Alexa+ track
- **Mini Challenges**: AWS Builder challenge & Open Source mini challenge

---

## Prerequisites

- **Rust**: stable toolchain (1.80+ recommended, managed via `rust-toolchain.toml`)
- **Node.js**: v20+ LTS (with `npm` v10+)
- **Git**
- **curl** (for endpoint testing)

---

## Project Structure

```
tabletalk/
├── backend/                  # Cargo workspace member (Axum + Tokio)
│   ├── Cargo.toml            # Backend dependencies
│   └── src/
│       └── main.rs           # Axum HTTP server with GET /health endpoint
├── frontend/                 # Vite + React + TypeScript web application
│   ├── src/
│   │   ├── components/ui/    # shadcn/ui components (Tailwind CSS)
│   │   ├── App.tsx           # TableTalk client interface
│   │   └── main.tsx          # Application entrypoint
│   ├── package.json
│   └── vite.config.ts
├── docs/
│   ├── FRICTION_LOG.md       # Developer experience and friction tracking
│   └── PRODUCT_FEEDBACK.md   # Feedback for MCP SDK, Streamable HTTP, AWS, and AI tools
├── .github/
│   └── workflows/
│       └── ci.yml            # CI workflow for backend and frontend
├── .gitignore                # Workspace git ignore rules
├── Cargo.toml                # Root workspace configuration
├── rust-toolchain.toml       # Pinned Rust stable toolchain
├── LICENSE                   # MIT License
└── README.md
```

---

## Setup & Run

### 1. Backend (Rust / Axum)

Run tests across the workspace:
```bash
cargo test
```

Run linter and formatting checks:
```bash
cargo fmt --check
cargo clippy -- -D warnings
```

Start the backend server (binds to `0.0.0.0:${PORT:-8080}`):
```bash
cargo run --bin tabletalk-backend
```

Verify the health check endpoint in a separate terminal:
```bash
curl http://localhost:8080/health
# Response: {"status":"ok"}
```

To configure a custom port:
```bash
PORT=9000 cargo run --bin tabletalk-backend
```

### 2. Frontend (React + Vite + TypeScript)

Install dependencies:
```bash
cd frontend
npm install
```

Start the local development server:
```bash
npm run dev
```
Open `http://localhost:5173` in your browser.

Build for production:
```bash
npm run build
```

---

## License

This project is licensed under the [MIT License](LICENSE).
