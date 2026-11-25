# Guide: the always-on host of the paper algo (Google Cloud e2-micro)

For the person (Luc). Time: about 45 minutes, one time. Ticket `T-20261121-algo-host-gcp`. The program sends
orders only to the Alpaca PAPER account (`CLAUDE.md`, "Purpose"; `docs/QUALITY-STANDARDS.md` section 8).

## 1. Make the machine (Google Cloud console, in a browser)
1. Open console.cloud.google.com, make a project `fa-algo`, and turn on "Compute Engine API".
2. Compute Engine → VM instances → Create instance:
   - Region `us-east1`, machine type `e2-micro` (the free tier: one e2-micro in us-west1, us-central1 or
     us-east1).
   - Boot disk: Debian 12, "Standard persistent disk", 30 GB (the free size).
   - Firewall: no HTTP or HTTPS. Create.
3. Click "SSH" on the line of the machine. A terminal opens in the browser. All next steps are in it.

## 2. Prepare the machine (copy each block into the SSH window)
```bash
sudo timedatectl set-timezone America/New_York
timedatectl          # "System clock synchronized: yes" must show
sudo useradd -m -s /bin/bash algo
sudo mkdir -p /home/algo/fa/rust/target/release /home/algo/fa/data/processed/algo
sudo chown -R algo:algo /home/algo/fa
```

## 3. Put the program on the machine
1. On GitHub (logged in), open the private store `louellet-git/fa-data`, folder `bin/`, and download
   `fa-linux-x86_64` (the Head of Technology puts each new build there, with its SHA-256 in `bin/SHA256SUMS`).
2. In the SSH window: the gear menu → "Upload file" → choose that file. Then:
```bash
sha256sum fa-linux-x86_64     # must equal the line in bin/SHA256SUMS
sudo install -o algo -g algo -m 755 fa-linux-x86_64 /home/algo/fa/rust/target/release/fa
sudo -u algo /home/algo/fa/rust/target/release/fa algo --help | head -3
```

## 4. The keys (your own PAPER keys; they stay on this machine only)
In the Alpaca web site, paper account → API keys → generate. Then (replace the two values):
```bash
sudo tee /etc/fa-algo.env >/dev/null <<'END'
ALPACA_PAPER_KEY_ID=PASTE_KEY_ID
ALPACA_PAPER_SECRET_KEY=PASTE_SECRET
APCA_API_KEY_ID=PASTE_KEY_ID
APCA_API_SECRET_KEY=PASTE_SECRET
FA_REPO=/home/algo/fa
END
sudo chmod 600 /etc/fa-algo.env
```
The same paper key reads the market data host (tested on 2026-11-25: a paper key gave HTTP 200 on
data.alpaca.markets). Do not put a live key here. Do not paste a key into a chat.

## 5. The service (each market day, 09:31 to 15:55 New York time)
```bash
sudo tee /etc/systemd/system/fa-algo.service >/dev/null <<'END'
[Unit]
Description=fa paper algo (Alpaca paper account)
After=network-online.target
[Service]
User=algo
WorkingDirectory=/home/algo/fa
EnvironmentFile=/etc/fa-algo.env
ExecStart=/home/algo/fa/rust/target/release/fa algo run --strategy demo --symbol SPY --broker alpaca-paper --until 15:55
Restart=on-failure
RestartSec=60
END
sudo tee /etc/systemd/system/fa-algo.timer >/dev/null <<'END'
[Unit]
Description=start fa algo on market days
[Timer]
OnCalendar=Mon..Fri 09:31 America/New_York
Persistent=false
[Install]
WantedBy=timers.target
END
sudo systemctl daemon-reload && sudo systemctl enable --now fa-algo.timer
systemctl list-timers fa-algo.timer
```
- The line has no `--send`: the program does a dry run and sends nothing. Add `--send` to `ExecStart` only
  after the Head of Quant Research says that step A passed its gate (then `sudo systemctl daemon-reload`).
- A market holiday: the timer still starts the program. Until the Head of Quant Research confirms a holiday
  check in the engine, make the kill file (section 6) the evening before a holiday, and remove it after.
- Logs: `journalctl -u fa-algo -n 50`. A test now: `sudo systemctl start fa-algo`.

## 6. Stop from the phone (the kill file)
Open the Google Cloud app (or the console in the phone browser) → the machine → SSH, then:
```bash
sudo -u algo touch /home/algo/fa/data/processed/algo/STOP
```
The program closes the positions and stops. To start again: `sudo -u algo rm /home/algo/fa/data/processed/algo/STOP`.

## 7. The daily summary to the private journal repo
1. On GitHub, make a private repo `louellet-git/fa-algo-journal`.
2. On the machine: `sudo -u algo ssh-keygen -t ed25519 -N "" -f /home/algo/.ssh/journal` and
   `sudo cat /home/algo/.ssh/journal.pub`. On GitHub: journal repo → Settings → Deploy keys → Add, paste the
   line, tick "Allow write access". This key can write this one repo only.
3. Tell the Head of Technology "journal ready". It adds the push of the daily summary to the service (the
   raw quotes stay on the machine; 30 days are kept).

## 8. Cost check
The machine is in the free tier. Google may charge the external IPv4 address (about US$3.65 a month). After day 1,
open Billing → Reports and tell the Head of Technology the amount; it writes it in the ticket.
