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

คำสั่ง `backup` ปกติและ `export` เขียนลง manual backup directory ส่วน `backup --automatic` ใช้ automatic backup directory สำหรับ systemd timer

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
auto_backup_dir = "~/zsh/backup/auto"
manual_backup_dir = "~/zsh/backup/manual"
```

ติดตั้ง config ตัวอย่างเมื่อยังไม่มี config เดิม:

```console
install -Dm600 config.toml "$HOME/.config/zsh-history-backup/config.toml"
```

สามารถเลือกไฟล์อื่นเฉพาะครั้งได้ด้วย `--config /absolute/path/config.toml` เมื่อระบุ `--config` ไฟล์นั้นต้องมีอยู่และอ่านได้ ทุก path ใน config ต้องเป็น absolute path หรือเริ่มด้วย `~/` โปรแกรมจะสร้าง backup directory ให้และตั้ง permission เป็น `0700`

## ตำแหน่ง backup และ log

ตำแหน่ง backup แยกตามผู้เรียก:

```text
~/zsh/backup/
├── auto/
└── manual/
```

ไฟล์จาก `export` อยู่ใน `manual/` เช่นเดียวกับ manual backup ส่วน log ยังคงแยกที่ `~/.local/state/zsh-history-backup/logs/backup.log` ไดเรกทอรีตั้ง permission เป็น `0700` ส่วน archive และ log เป็น private file โปรแกรมไม่บันทึกเนื้อหาคำสั่งลง log

หากเปลี่ยน `auto_backup_dir` ภายหลัง ต้องเปลี่ยน `ReadWritePaths` ใน `systemd/zsh-history-backup.service` ให้เป็น path เดียวกัน เพื่อคง filesystem sandbox และให้ timer เขียนปลายทางได้

## Build และทดลองแบบ manual

```console
make build
./target/release/zsh-history-backup backup --name first-manual-run
```

## ติดตั้ง systemd user timer

โปรเจกต์มี unit ที่เรียก backup ทุกวันเวลา `00:00:00` ตาม timezone ท้องถิ่น และมี `Persistent=true` เพื่อให้ systemd เรียกงานที่พลาดไประหว่างปิดเครื่องหลังกลับมาเปิดเครื่องอีกครั้ง

ติดตั้ง, reload systemd user manager และ enable/start timer ทันที:

```console
make install
```

`make install` จะไม่เขียนทับ config ที่มีอยู่ และจะเรียก `systemctl --user daemon-reload` ตามด้วย `enable --now` ให้ ตรวจสถานะด้วย:

```console
make status
```

`make enable-timer` เป็น alias ของ `make install`

คำสั่งที่ Makefile เรียกภายในมีดังนี้:

```console
install -d -m700 "$HOME/zsh/backup/auto"
install -d -m700 "$HOME/zsh/backup/manual"
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
make disable-timer
```

## ขอบเขตความปลอดภัย

- เปิด history file ที่กำหนดด้วยสิทธิ์ read-only และไม่แก้ไขหรือลบต้นฉบับ
- อ่านตามขนาดไฟล์ ณ ตอนเปิด หากไฟล์ถูก truncate ระหว่างอ่าน จะไม่ publish archive ที่ไม่สมบูรณ์
- สร้างไฟล์ผ่าน temporary file แล้ว publish แบบ no-clobber จึงไม่เขียนทับ archive เดิม
- export เก็บ entry แบบ raw bytes จึงไม่บังคับให้ history ทั้งไฟล์เป็น UTF-8
- systemd service ใช้ `ProtectHome=read-only`, เปิดเขียนเฉพาะ `~/zsh/backup/auto`, ใช้ `NoNewPrivileges=yes` และ private state directory
