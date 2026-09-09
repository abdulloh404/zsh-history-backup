# zsh-history-backup

CLI ภาษา Rust สำหรับสำรอง Zsh history โดยเปิดต้นฉบับแบบ read-only เท่านั้น ผลลัพธ์เป็น gzip, ไม่เขียนทับไฟล์ backup เดิม และใช้ timestamp จากรูปแบบ `EXTENDED_HISTORY` เมื่อต้อง export ประวัติของวันใดวันหนึ่ง

## คำสั่ง

สร้าง snapshot ทั้งไฟล์ทันที:

```console
zsh-history-backup backup
zsh-history-backup backup --name before-upgrade
```

export เฉพาะ history ของวันย้อนหลัง หรือวันที่ระบุ:

```console
zsh-history-backup export --days-ago 3
zsh-history-backup export --days-ago 3 --name project-debug
zsh-history-backup export --date 2026-09-06 --name sunday
```

`--days-ago 0` หมายถึงวันนี้ วันที่ของแต่ละ entry คำนวณด้วย timezone ท้องถิ่นของเครื่อง หากวันที่นั้นไม่มีคำสั่ง จะได้ gzip ที่ถูกต้องแต่ไม่มี entry

ชื่อที่ส่งผ่าน `--name` รองรับตัวอักษร Unicode, ตัวเลข, ช่องว่าง, จุด, `-` และ `_` โดยช่องว่างจะถูกเปลี่ยนเป็น `-` และไม่อนุญาต path separator หรือชื่อซ่อน

## Config

config แยกจาก backup และ log ตามมาตรฐาน XDG:

```text
~/.config/zsh-history-backup/config.toml
```

ไฟล์นี้เป็น optional หากไม่มีโปรแกรมจะใช้ path ปริยายตามเดิม ตัวอย่างอยู่ที่ `config.toml`:

```toml
history_file = "~/.zsh_history"
backup_dir = "backups"
```

ติดตั้ง config ตัวอย่างเมื่อยังไม่มี config เดิม:

```console
install -Dm600 config.toml "$HOME/.config/zsh-history-backup/config.toml"
```

สามารถเลือกไฟล์อื่นเฉพาะครั้งได้ด้วย `--config /absolute/path/config.toml` เมื่อระบุ `--config` ไฟล์นั้นต้องมีอยู่และอ่านได้ `history_file` ต้องเป็น absolute path หรือเริ่มด้วย `~/` ส่วน `backup_dir` เป็น path ภายใน XDG State ของแอป เช่น `backups` หรือ `daily/backups` โปรแกรมจะสร้างโฟลเดอร์ให้และตั้ง permission เป็น `0700`

## ตำแหน่ง backup และ log

ค่าปริยายของ backup เป็นไปตาม `backup_dir` ภายใน XDG State Directory:

```text
~/.local/state/zsh-history-backup/
├── backups/
├── exports/
└── logs/backup.log
```

config ไม่ถูกเก็บในตำแหน่งนี้ ถ้ากำหนด `XDG_STATE_HOME` เป็น absolute path โปรแกรมจะใช้ตำแหน่งนั้นแทน ไดเรกทอรีตั้ง permission เป็น `0700` ส่วน archive และ log เป็น private file โปรแกรมไม่บันทึกเนื้อหาคำสั่งลง log

## Build และทดลองแบบ manual

```console
cargo build --release --locked
./target/release/zsh-history-backup backup --name first-manual-run
```

## ติดตั้ง systemd user timer

โปรเจกต์มี unit ที่เรียก backup ทุกวันเวลา `00:00:00` ตาม timezone ท้องถิ่น และมี `Persistent=true` เพื่อให้ systemd เรียกงานที่พลาดไประหว่างปิดเครื่องหลังกลับมาเปิดเครื่องอีกครั้ง

```console
install -Dm755 target/release/zsh-history-backup "$HOME/.local/bin/zsh-history-backup"
install -Dm644 systemd/zsh-history-backup.service "$HOME/.config/systemd/user/zsh-history-backup.service"
install -Dm644 systemd/zsh-history-backup.timer "$HOME/.config/systemd/user/zsh-history-backup.timer"
systemctl --user daemon-reload
systemctl --user enable --now zsh-history-backup.timer
systemctl --user list-timers zsh-history-backup.timer
```

ตรวจ log ของโปรแกรมได้ที่ `~/.local/state/zsh-history-backup/logs/backup.log` และตรวจ journal ได้ด้วย:

```console
journalctl --user -u zsh-history-backup.service
```

การถอน timer ไม่กระทบ backup ที่มีอยู่:

```console
systemctl --user disable --now zsh-history-backup.timer
```

## ขอบเขตความปลอดภัย

- เปิด history file ที่กำหนดด้วยสิทธิ์ read-only และไม่แก้ไขหรือลบต้นฉบับ
- อ่านตามขนาดไฟล์ ณ ตอนเปิด หากไฟล์ถูก truncate ระหว่างอ่าน จะไม่ publish archive ที่ไม่สมบูรณ์
- สร้างไฟล์ผ่าน temporary file แล้ว publish แบบ no-clobber จึงไม่เขียนทับ archive เดิม
- export เก็บ entry แบบ raw bytes จึงไม่บังคับให้ history ทั้งไฟล์เป็น UTF-8
- systemd service ใช้ `ProtectHome=read-only`, `NoNewPrivileges=yes` และ private state directory
