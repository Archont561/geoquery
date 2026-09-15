---
id: infrastructure/deployment
title: Deployment Targets
category: infrastructure
tags: [deployment, edge, Docker, WASM, Cloudflare, Deno, serverless, binary]
refs: [project/architecture, infrastructure/storage, infrastructure/monorepo, interfaces/http]
status: draft
created: 2025-07-11
updated: 2025-07-11
---

# Deployment Targets

## Core Principle

The same architecture runs everywhere without changing the query model:

```
single binary
    ↓
local machine
    ↓
Docker
    ↓
Node server
    ↓
edge API
    ↓
managed SaaS
```

---

## Deployment Matrix

| Target | Tier | Storage | Query Execution | Phase |
|--------|------|---------|----------------|-------|
| **Native binary** | 0–2 | Local FS | Local + remote | 1 |
| **Docker** | 0–3 | Volume mount | Local + remote | 3 |
| **Cloudflare Workers** | 0 | Stateless | Remote only | 6 |
| **Deno Deploy** | 0 | Stateless | Remote only | 6 |
| **Vercel Edge** | 0 | Stateless | Remote only | 6 |
| **Bun** | 0–1 | SQLite (bun:sqlite) | Remote + local | 6 |
| **Node.js** | 0–2 | SQLite / DuckDB | Remote + local | 3 |
| **WASM (browser)** | 0 | IndexedDB / OPFS | Remote + WASM local | 6 |

---

## Native Binary

The primary deployment for developers and servers.

```bash
cargo build --release --package geoquery-cli
cargo build --release --package geoquery-http

# CLI
./target/release/geoquery query --bbox ...

# HTTP server
./target/release/geoquery-http --port 8080
```

Cross-compilation targets:
- `x86_64-unknown-linux-gnu` (Linux servers)
- `aarch64-unknown-linux-gnu` (ARM servers, Raspberry Pi)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-pc-windows-msvc` (Windows)

---

## Docker

### Multi-Stage Build with `cargo-chef`

```dockerfile
# Stage 1: Plan
FROM rust:1.85 AS planner
RUN cargo install cargo-chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies (cached layer)
FROM rust:1.85 AS cacher
RUN cargo install cargo-chef
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build application
FROM rust:1.85 AS builder
WORKDIR /app
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY . .
RUN cargo build --release --package geoquery-http

# Stage 4: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/geoquery-http /usr/local/bin/
EXPOSE 8080
CMD ["geoquery-http", "--port", "8080"]
```

### Docker Compose (Development)

```yaml
services:
  geoquery:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./geoquery-data:/data
    environment:
      GEOQUERY_REGISTRY: /data/registry

  postgis:  # Optional, Tier 3
    image: postgis/postgis:16-3.4
    environment:
      POSTGRES_PASSWORD: geoquery
    ports:
      - "5432:5432"
```

---

## Edge Deployment

### Architecture

**Do not make edge runtimes responsible for executing arbitrary GIS
queries.** Instead:

```
                 Edge
                  │
           Geoquery HTTP
                  │
        ┌─────────┴─────────┐
        │                   │
    metadata             planner
     index
        │                   │
        └─────────┬─────────┘
                  │
             remote APIs
```

The edge is excellent for:
- Resource registry (small metadata index)
- Metadata search
- Semantic search (small embedding index)
- Query planning
- Caching
- Authentication
- MCP/HTTP interface

Heavyweight spatial execution happens in:
- PostGIS (Tier 3)
- DuckDB (Tier 2)
- Remote STAC/OGC APIs (Tier 0)

### Cloudflare Workers

```typescript
// Future: geoquery-wasm compiled to Cloudflare Workers
export default {
  async fetch(request: Request, env: Env) {
    const query = await request.json();
    // WASM planner runs locally
    // Remote federation via fetch() to STAC/OGC APIs
    const results = await geoquery.execute(query);
    return Response.json(results);
  }
};
```

**Constraints:**
- No PostGIS (obviously)
- No DuckDB (too large for Workers)
- Limited CPU time (50ms free, 30s paid)
- KV or D1 for small metadata index
- Remote federation only for data queries

### WASM (Browser)

```
geoquery-wasm
    │
    ├── Core query planner (compiled to .wasm)
    ├── Metadata search (in-memory or IndexedDB)
    ├── DuckDB-WASM (optional, for local GeoParquet)
    └── Remote federation (via fetch API)
```

**Constraints:**
- No `reqwest` in WASM (use `web-sys::fetch`)
- No filesystem (use OPFS or IndexedDB)
- `geo` crate is WASM-compatible
- `rstar` is WASM-compatible
- `tantivy` has WASM support (limited)

---

## Security Considerations

### SSRF Protection

Never allow arbitrary remote URLs to become unrestricted SSRF targets
in hosted deployments.

| Protection | Implementation |
|-----------|---------------|
| Allowlists | Only query registered source URLs |
| Private network protection | Block 10.x, 172.16.x, 192.168.x |
| DNS rebinding | Resolve and validate before connecting |
| Redirect validation | Follow redirects only to allowed domains |
| Credential isolation | Per-source credentials, no cross-leak |
| Request timeouts | Per-source timeout (default 5s) |
| Response size limits | Cap response body (default 10MB) |
| Rate limits | Per-source and per-user rate limiting |

### Authentication

Credentials MUST NOT be stored inside public resource manifests.

```yaml
# resource.yaml
authentication:
  profile: copernicus-production  # Reference, not secret
```

The actual secret is supplied by the runtime via `AuthResolver`:
- Environment variables
- Cloud provider metadata (AWS IAM, GCP SA)
- Vault / Secrets Manager
- Config file (`~/.geoquery/credentials.yaml`)

→ See [extensions/extension-points](../extensions/extension-points.md) Layer 6

---

## Caching Strategy

| What | Cache | TTL | Invalidation |
|------|-------|-----|-------------|
| Service capabilities | Memory + disk | 1 hour | Re-discover on miss |
| Resource metadata | Registry DB | Until updated | `geoquery add` refresh |
| Catalog discovery | Memory | 10 min | Manual refresh |
| Static resources | Disk | 24 hours | Content hash |
| Query responses | Optional (Redis) | Configurable | Explicit policy |
| STAC search results | GeoParquet | Configurable | `geoquery cache --refresh` |

**Do not cache live geospatial results indefinitely without explicit
policy.** Respect HTTP caching headers where available.

---

## Related Files

- [project/architecture](../project/architecture.md) — Component diagram
- [infrastructure/storage](storage.md) — Storage tiers per deployment
- [infrastructure/monorepo](monorepo.md) — Build configuration
- [interfaces/http](../interfaces/http.md) — HTTP server deployment
- [interfaces/mcp](../interfaces/mcp.md) — MCP server deployment
- [extensions/extension-points](../extensions/extension-points.md) — Auth and security extensions
