# 🤖 Indodax MCP Server: Full Feature Guide

Server ini adalah jembatan antara AI Agent (ChatGPT/Claude) dengan bursa Indodax menggunakan **Model Context Protocol (MCP)**. Dengan server ini, AI Anda bukan sekadar "ngobrol", tapi bisa memantau pasar, mengelola saldo, hingga melakukan trading otomatis.

---

## 🛠 Fitur Berdasarkan Kategori (Service Groups)

Server ini dibagi menjadi beberapa grup layanan. Anda bisa mengaktifkan grup tertentu menggunakan flag `--groups`.

### 1. 📈 Market Data (Public)
Digunakan oleh AI untuk menganalisis pasar tanpa perlu API Key.
- `ticker`: Ambil harga terkini & statistik 24 jam untuk pair tertentu (misal: `btc_idr`).
- `orderbook`: Lihat kedalaman pasar (antrian beli & jual).
- `trades`: Lihat histori perdagangan terbaru di pasar.
- `ohlc`: Ambil data candlestick untuk analisis teknikal (Interval 1m hingga 1w).
- `summaries`: Ringkasan statistik seluruh koin dalam 24 jam terakhir.
- `price_increments`: Cek fraksi harga (tick size) agar order tidak ditolak.

### 2. 💰 Account & Balance (Private)
Membantu AI mengetahui kondisi keuangan Anda.
- `account_info`: Informasi profil akun dan izin API Key.
- `balance`: Cek saldo seluruh aset (hanya menampilkan yang saldonya > 0).
- `open_orders`: List antrian order yang belum selesai (pending).
- `order_history`: Riwayat order masa lalu.
- `trans_history`: Riwayat deposit dan penarikan (IDR & Crypto).

### 3. ⚡ Trading (Private - Dangerous)
Memberikan kemampuan AI untuk bertransaksi.
- `buy_order`: Pasang order beli (Limit atau Market).
- `sell_order`: Pasang order jual (Limit atau Market).
- `cancel_order`: Membatalkan order spesifik berdasarkan ID.
- `cancel_all_orders`: Membersihkan semua antrian order (bisa difilter per pair).

### 4. 📝 Paper Trading (Simulasi)
Fitur unggulan untuk testing AI tanpa risiko kehilangan uang asli.
- `paper_init`: Memulai akun simulasi dengan saldo IDR & BTC awal.
- `paper_balance`: Cek saldo virtual.
- `paper_buy` / `paper_sell`: Simulasi trading menggunakan harga real-time tapi saldo virtual.
- `paper_status`: Cek performa trading simulasi (Profit/Loss).

### 5. 🔔 Alert & Automation
- `alert_add`: AI bisa memasang pengingat harga (misal: "Beri tahu saya jika BTC tembus 1 Milyar").
- `alert_list`: Lihat daftar alert yang sedang aktif.

---

## 💡 Prompts (Alur Kerja Otomatis)
Server ini menyediakan **Prompts**, yaitu template instruksi yang memudahkan Anda memerintah AI:

1. **`check_portfolio`**: AI akan otomatis mengecek saldo, merangkum aset terbesar, dan melaporkan antrian order Anda.
   - *Cara pakai:* "Check my portfolio summary."
2. **`analyze_market`**: AI akan mengambil data ticker, orderbook, dan trade history lalu memberikan opini teknikal.
   - *Cara pakai:* "Analyze the BTC market for me."
3. **`create_order`**: Membantu AI membuat parameter order yang valid sebelum dieksekusi.

---

## 📂 Resources (Data Mentah)
AI memiliki akses langsung ke data berikut untuk referensi konteks:
- `config://current`: Konfigurasi API yang sedang digunakan.
- `pairs://list`: Daftar lengkap seluruh koin yang ada di Indodax.
- `paper://state`: Kondisi terkini dari akun simulasi.

---

## 🎮 Contoh Perintah ke AI
Setelah terhubung, Anda bisa mencoba bertanya seperti ini:

> *"Berapa harga Bitcoin sekarang? Tolong buatkan analisis singkat apakah layak beli berdasarkan orderbook."*

> *"Tampilkan saldo saya yang paling banyak nilainya dalam IDR."*

> *"Buka akun paper trading dengan saldo 100 juta IDR, lalu beli ETH pakai semua saldo itu di harga pasar."*

> *"Cek semua order terbuka saya, jika ada yang sudah lebih dari 2 hari, batalkan semuanya."*

---

## 🔐 Keamanan (Isolasi)
- **Flag `--allow-dangerous`**: Secara default, AI tidak bisa melakukan `buy`/`sell` kecuali Anda menjalankan server dengan flag ini.
- **Isolasi Multi-user**: Jika dideploy di cloud, setiap user yang memasukkan API Key berbeda akan memiliki sandbox yang terpisah total.

---
*Dokumentasi ini dibuat otomatis untuk Indodax CLI MCP Server.*
