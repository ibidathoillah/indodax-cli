# 🤖 Indodax MCP Server: Full Feature Guide

Server ini adalah jembatan antara AI Agent (ChatGPT/Claude) dengan bursa Indodax menggunakan **Model Context Protocol (MCP)**. Dengan server ini, AI Anda bisa memantau pasar, mengelola saldo, hingga melakukan trading otomatis.

---

## 🚀 Quick Start (Cara Menjalankan)

### 1. Mode Standar (Claude Desktop / IDE)
Gunakan mode ini jika Anda ingin menghubungkan MCP ke aplikasi desktop.
```bash
# Build binary terlebih dahulu
cargo build --release --features "mcp cli"

# Jalankan via STDIO
./target/release/indodax-cli mcp serve --groups all
```

### 2. Mode HTTP Bridge (ChatGPT Mobile / Glama)
Gunakan mode ini untuk akses via HP atau layanan cloud.

#### Setup via Cloudflare Tunnel (Custom Domain)
1. Set token Anda di environment variable:
   ```bash
   export CLOUDFLARE_TUNNEL_TOKEN="your-token-here"
   ```
2. Jalankan menggunakan Docker Compose:
   ```bash
   docker-compose up -d
   ```
Server akan otomatis terhubung ke `https://indodax-mcp.tep2.in`.

---

## 🛠 Fitur Berdasarkan Kategori

### 1. 📈 Market Data (Public)
- `ticker`: Harga terkini & statistik 24 jam.
- `orderbook`: Antrian beli & jual (market depth).
- `trades`: Histori perdagangan terbaru.
- `ohlc`: Data candlestick (1m, 5m, 1h, dst).
- `summaries`: Ringkasan seluruh pasar.

### 2. 💰 Account & Balance (Private)
- `account_info`: Profil akun & izin API.
- `balance`: Saldo aset (hanya aset aktif).
- `open_orders`: Daftar antrian order pending.
- `order_history`: Riwayat order masa lalu.

### 3. ⚡ Trading (Dangerous Operations)
*Catatan: Membutuhkan flag `--allow-dangerous` atau parameter `acknowledged: true`.*
- `buy_order`: Eksekusi beli (Limit/Market).
- `sell_order`: Eksekusi jual (Limit/Market).
- `cancel_order`: Batalkan order tertentu.
- `cancel_all_orders`: Bersihkan semua antrian order.

### 4. 📝 Paper Trading (Simulasi)
- `paper_init`: Buat akun virtual dengan saldo simulasi.
- `paper_balance`: Cek saldo virtual.
- `paper_buy` / `paper_sell`: Trading simulasi tanpa risiko.

---

## ⚙️ Konfigurasi Claude Desktop

Tambahkan ini ke file `claude_desktop_config.json` Anda:

```json
{
  "mcpServers": {
    "indodax": {
      "command": "/path/to/indodax-cli",
      "args": ["mcp", "serve", "--groups", "all"],
      "env": {
        "INDODAX_API_KEY": "YOUR_KEY",
        "INDODAX_API_SECRET": "YOUR_SECRET"
      }
    }
  }
}
```

---

## 🔐 Keamanan & Isolasi

1. **Safety First**: Fitur trading dinonaktifkan secara default. Gunakan `--allow-dangerous` untuk mengizinkan AI melakukan transaksi.
2. **Multi-User Isolation**: Jika dijalankan dalam mode HTTP, server mengisolasi data (seperti paper trading state) berdasarkan `x-api-key` yang dikirim di header. Setiap user memiliki sandbox yang berbeda.
3. **Logs**: Semua log sistem dikirim ke `stderr`, sehingga tidak mengganggu jalur komunikasi data `stdout` (JSON-RPC).

---

## 🎮 Contoh Perintah ke AI
- *"Berapa harga BTC sekarang? Apakah market sedang bullish?"*
- *"Tampilkan saldo Indodax saya dalam bentuk tabel."*
- *"Gunakan akun paper trading untuk simulasi beli ETH senilai 10 juta IDR."*
- *"Batalkan semua open order saya di pair btc_idr."*

---
*Dokumentasi ini adalah bagian dari proyek [Indodax CLI](https://github.com/ibidathoillah/indodax-cli).*
