# ⏰ Ingatin

**Ingatin** (Indonesian for *"Remind me"*) is a lightweight, AI-powered WhatsApp bot designed to manage personal academic tasks and event reminders. Instead of relying on rigid command syntaxes, Ingatin uses an LLM to understand natural language inputs, seamlessly extracting task details and deadlines, and scheduling automated WhatsApp alerts.

Built with an emphasis on low memory footprint and high performance, this monolithic backend runs entirely on **Rust** and **SQLite**, making it highly efficient for cheap VPS deployments.

## ✨ Features

- **🧠 Natural Language Processing:** Just chat with the bot naturally (e.g., *"Remind me to submit my networking assignment tomorrow at 10 AM"*), and the LLM handles the parsing.
- **⚡ Blazing Fast & Lightweight:** Powered by Rust (Axum) and SQLite. No heavy message brokers (like Redis/RabbitMQ) required.
- **🕒 Native Background Scheduler:** Utilizes `tokio::time::interval` for an efficient, non-blocking asynchronous polling system to trigger reminders.
- **💬 Direct WhatsApp Integration:** Uses WAHA (WhatsApp HTTP API) for real-time inbound webhooks and outbound notifications.
- **🐳 Docker Ready:** Easily deployable using a single `docker-compose.yml`.

## 🛠️ Tech Stack

- **Backend:** [Rust](https://www.rust-lang.org/)
- **Web Framework:** [Axum](https://github.com/tokio-rs/axum)
- **Database:** [SQLite](https://sqlite.org/) (via [SQLx](https://github.com/launchbadge/sqlx))
- **WhatsApp API:** [WAHA (WhatsApp HTTP API)](https://waha.devlike.pro/)
- **AI / LLM:** Gemini API (potentially)

## 🏗️ Architecture Flow

1. **Inbound:** You send a message via WhatsApp. WAHA triggers a webhook to the Rust Axum server.
2. **Extraction:** The server forwards the text to an LLM with a specific system prompt to extract structured JSON (Task Name, Deadline, Reminder Time).
3. **Storage:** The parsed data is stored safely in a local SQLite `.db` file.
4. **Polling:** A Tokio background task ticks every 60 seconds, querying SQLite for due reminders.
5. **Outbound:** When a schedule is met, the server calls the WAHA API to push a reminder message back to your WhatsApp.

## 🚀 Quick Start

### Prerequisites
- [Rust](https://rustup.rs/) installed.
- [Docker & Docker Compose](https://docs.docker.com/get-docker/) (for running WAHA).
- An active API Key from your chosen LLM provider.

### Setup Instructions

1. **Clone the repository:**
   ```bash
   git clone [https://github.com/akmmp241/ingatin.git](https://github.com/akmmp241/ingatin.git)
   cd ingatin
