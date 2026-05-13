# 🚀 WorldVPN Backend Server

<div align="center">

<br/>

```
 ██████╗  █████╗  ██████╗██╗  ██╗███████╗███╗   ██╗██████╗ 
 ██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝████╗  ██║██╔══██╗
 ██████╔╝███████║██║     █████╔╝ █████╗  ██╔██╗ ██║██║  ██║
 ██╔══██╗██╔══██║██║     ██╔═██╗ ██╔══╝  ██║╚██╗██║██║  ██║
 ██████╔╝██║  ██║╚██████╗██║  ██╗███████╗██║ ╚████║██████╔╝
 ╚═════╝ ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝╚═════╝ 
```

**Central Orchestration & Transparency API**

<br/>

![Rust](https://img.shields.io/badge/Language-Rust-orange?style=flat-square&logo=rust)
![Axum](https://img.shields.io/badge/Framework-Axum-blue?style=flat-square)
![PostgreSQL](https://img.shields.io/badge/Database-PostgreSQL-336791?style=flat-square&logo=postgresql)
![Dbmate](https://img.shields.io/badge/Migrations-Dbmate-green?style=flat-square)

</div>

## 📖 Overview
The `backend` is the central brain of WorldVPN. It handles node registration, session management, and the public transparency dashboard. Unlike traditional VPNs, the backend **never** sees your traffic—it only facilitates discovery and coordination.

## 🛠 Tech Stack
- **Web Framework**: [Axum](https://github.com/tokio-rs/axum) (High performance, async-first).
- **Database**: PostgreSQL with [sqlx](https://github.com/launchbadge/sqlx).
- **Migration Engine**: [Dbmate](https://github.com/amacneil/dbmate).
- **Real-time**: WebSocket-ready for transparency updates.

## 📂 Structure
- `/src/api`: REST Endpoints (Auth, Nodes, Credits, Transparency).
- `/src/services`: Background workers (VPNGate sync, pruning, status monitoring).
- `/migrations`: Versioned SQL schema using Dbmate.

## 🚀 Getting Started
1. Install [Dbmate](https://github.com/amacneil/dbmate).
2. Set `DATABASE_URL` in `.env`.
3. Run migrations:
   ```bash
   dbmate up
   ```
4. Start the server:
   ```bash
   cargo run
   ```

---
**WorldVPN** · Private. Decentralized. Transparent.
