# ⏰ Ingatin

**Ingatin** (Indonesian: *"Remind me"*) is an AI-powered WhatsApp reminder bot that understands natural language. Simply chat with it like you would a friend — describe your task, deadline, and when you'd like to be reminded — and it handles the rest automatically.

Built with **Rust** for maximum performance and **SQLite** for zero-ops persistence, Ingatin is designed to run efficiently on minimal infrastructure (including cheap VPS instances) without sacrificing reliability.

---

## ✨ Features

- **🧠 Natural Language Understanding** — No rigid commands. Just send messages like *"Ingatkan aku untuk submit tugas jaringan besok jam 10 pagi"* and the LLM extracts the task, deadline, and reminder schedule automatically.
- **⚡ Blazing Fast & Lightweight** — Powered by Rust (Axum) and SQLite. No Redis, no RabbitMQ, no heavy dependencies.
- **🔁 Configurable Background Scheduler** — Uses `tokio::time::interval` for non-blocking async polling. Tick intervals are configurable via environment variables.
- **💬 WhatsApp Integration** — Seamless two-way messaging through [WAHA (WhatsApp HTTP API)](https://waha.devlike.pro/) — receives inbound webhooks and pushes outbound notifications.
- **🤖 AI-Generated Reminder Messages** — Reminder notifications are dynamically composed by the LLM, creating friendly and context-aware messages rather than generic alerts.
- **📋 Multi-Reminder Support** — A single task can have multiple reminder schedules (e.g., *"ingatkan H-1 dan 2 jam sebelumnya"*).

---

## 🛠️ Tech Stack

| Layer | Technology                                                                       |
|---|----------------------------------------------------------------------------------|
| **Language** | [Rust](https://www.rust-lang.org/)                                               |
| **Web Framework** | [Axum](https://github.com/tokio-rs/axum) 0.8                                     |
| **Async Runtime** | [Tokio](https://tokio.rs/)                                                       |
| **Database** | [SQLite](https://sqlite.org/) via [SQLx](https://github.com/launchbadge/sqlx) 0.8 |
| **AI / LLM** | [Google Gemini API](https://ai.google.dev/)                                      |
| **WhatsApp API** | [WAHA](https://waha.devlike.pro/)                                                |
| **HTTP Client** | [Reqwest](https://github.com/seanmonstar/reqwest)                                |
| **Logging** | [tracing](https://github.com/tokio-rs/tracing) + tracing-subscriber              |

---

## 🏗️ Architecture

```
┌──────────────┐     Webhook      ┌─────────────────────────────────────────┐
│   WhatsApp   │ ──────────────▶  │              Axum Server                │
│    (User)    │                  │                                         │
│              │ ◀──────────────  │  ┌─────────┐   ┌────────────────────┐  │
└──────────────┘   Push Message   │  │ Handler │──▶│  Gemini Client     │  │
                                  │  │(Webhook)│   │  (NLU Extraction)  │  │
                                  │  └────┬────┘   └────────────────────┘  │
                                  │       │                                │
                                  │       ▼                                │
                                  │  ┌─────────┐   ┌────────────────────┐  │
                                  │  │  Repo   │──▶│  SQLite Database   │  │
                                  │  │ (SQLx)  │   │  (tasks/reminders) │  │
                                  │  └─────────┘   └────────────────────┘  │
                                  │       ▲                                │
                                  │       │ poll                           │
                                  │  ┌─────────┐   ┌────────────────────┐  │
                                  │  │Scheduler│──▶│  Gemini Client     │  │
                                  │  │(Runner) │   │  (Msg Generation)  │  │
                                  │  └────┬────┘   └────────────────────┘  │
                                  │       │                                │
                                  │       ▼                                │
                                  │  ┌──────────────────┐                  │
                                  │  │  WAHA Client     │                  │
                                  │  │  (Send WhatsApp) │                  │
                                  │  └──────────────────┘                  │
                                  └─────────────────────────────────────────┘
```

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (1.9x)
- A running [WAHA](https://waha.devlike.pro/) instance (self-hosted via Docker)
- A [Google Gemini API Key](https://ai.google.dev/)

### 1. Clone the Repository

```bash
git clone https://github.com/akmmp241/ingatin.git
cd ingatin
```

### 2. Configure Environment Variables

Create a `.env` file in the project root:

```env
# Database
DATABASE_URL=sqlite://database/database.db

# Google Gemini
GEMINI_API_KEY=your_gemini_api_key_here
GEMINI_MODEL_TYPE=gemini-2.0-flash

# WAHA (WhatsApp HTTP API)
WAHA_API_URL=http://localhost:3000
WAHA_API_KEY=your_waha_api_key
WAHA_SESSION=default

# Scheduler (optional, defaults to 60 seconds)
TICK_INTERVAL=60
```

### 3. Initialize the Database

```bash
sqlite3 database/database.db < database/schema.sql
```

### 4. Run the Application

```bash
cargo run
```

The server will start on `http://0.0.0.0:3000` with the background scheduler running concurrently.

### 5. Configure WAHA Webhook

Point your WAHA instance's webhook URL to:

```
http://<your-server-ip>:3000/webhook/waha
```

---

## 📄 License

This project is licensed under the **Apache License 2.0** — see the [LICENSE](LICENSE) file for details.
