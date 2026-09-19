# Burnr — Quickstart

Burnr is a lightweight GUI for optical discs, built on libburn and libisofs
(both runtime-loaded from your system — nothing is compiled in).

## Start in three steps

1. **Scan** — Burnr lists your drives on the left; 🔄 rescans.
2. **Inspect media** — click **🔍 Inspect media** on the media card to read
   disc status, profile, capacity, TOC and speeds.
3. **Pick an island** — **🔥 Burning**, **💾 Disc image (data)** or
   **Maintenance** (on the media card).

## Burn an ISO file

Island **🔥 Burning** → Source: *ISO file* → choose the file → **🔥 Burn**.

## Burn files & folders

Source: *Files* → add files/folders, set a volume name →
**🔥 Compose & burn**. Built with libisofs (Rock Ridge + Joliet) and burned
directly — no intermediate file.

## Copy a disc to an ISO file

Island **💾 Disc image (data)** → choose the output path → **💾 Make copy**.
Data media only (no CD audio).

## Multi-session (write-once media)

Set **Multi-session: Yes** in Settings → Burning. On appendable media
(CD-R, DVD±R, BD-R) the existing session is imported and new files are added
at the next writable address.

## Erase / restore attempt

- CD-RW and sequential DVD-RW: **🧽 Erase** (quick / full).
- Troubled media: **🛠 Restore attempt** (advanced, collapsed) — a rescue
  re-format. The outcome depends on drive and firmware; a failed attempt is
  risk-free for the data.

## Tips

- **Automount blocks the drive?** Turn off *Exclusive open* in
  Settings → Device and rescan.
- **Simulation** (laser off) only works on write-once media; BD media,
  DVD+RW, DVD-RAM and formatted DVD-RW can never simulate.
- Speed, write mode, padding, file timestamps and eject behaviour live in
  **Settings**.
- The **Log** panel (bottom) explains every step; session logs are saved
  under `~/.local/share/burnr/logs/`.
- Language and color theme: top bar (English / Nederlands / Deutsch,
  Dark / Soft / Light).