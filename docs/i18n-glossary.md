# i18n Glossary & Translation Notes

This document preserves the terminology and style of the original Dutch
interface (the app was written in nl-NL before the en-US conversion) and
defines the process for the i18n work in step 10b.

## Purpose

When the interface is converted to en-US (source language), every existing
Dutch string must be carried over into the typed text catalogue's `nl-NL`
entries — not re-translated from scratch. The catalogue is the destination
of the Dutch corpus; git history alone is not a practical source.

## Process for the en-US sweep

1. Convert strings file by file (panels first, then worker/app log lines).
2. For **every** string: write the en-US text in the catalogue and copy the
   Dutch original (adjusted where the en-US rewording changed meaning) into
   the `nl-NL` entry.
3. Strings invented during the en-US sweep that have no Dutch predecessor:
   leave the `nl-NL` entry as `todo` — they get translated in the nl-NL
   phase, using this glossary for consistency.
4. Never delete a Dutch string without recording it. If a string becomes
   obsolete, note that in the catalogue comment.

## Terminology (nl-NL → en-US)

| Dutch | English | Notes |
| --- | --- | --- |
| station | drive | |
| stationscan | drive scan | |
| media inspecteren | inspect media | |
| grab / grabben | grab | technical term, same in both |
| Branden | Burning | panel/island name |
| Schijfkopie | Disc image | panel/island name |
| schijfkopie lezen | read disc (copy to file) | |
| wissen (snel/volledig) | erase (quick/full) | |
| herstelpoging | restore attempt | re-branded from "formatteren"/format; see below |
| defect management | defect management | same |
| certificering | certification | |
| multi-session | multi-session | same |
| schrijfmodus | write mode | |
| buffer-underrun-beveiliging | buffer underrun protection | |
| overburn | overburn | same |
| uitwerpen | eject | |
| aankoppelen / aangekoppeld | mount / mounted | |
| leesbaar | readable | |
| herbeschrijfbaar | rewritable | |
| overschrijfbaar | overwritable | |
| schrijf-eenmalig | write-once | |
| leeg | blank | libburn declares BD-RE/DVD+RW always blank |
| vol / afgesloten | full / closed | |
| onvolledig | appendable | libburn term: extra sessions possible |
| mediacode | media code | |
| boektype | book type | |
| inhoud (TOC) | table of contents (TOC) | |
| snelheid | speed | |
| herstelpoging geslaagd / mislukt | restore attempt succeeded / failed | |

## Style notes

- The app addresses the user implicitly; buttons use short imperatives.
- Em-dash "—" with spaces, used throughout — keep this in en-US too.
- Technical terms (grab, NWA, FIFO, TOC, MMC) stay untranslated in nl-NL;
  en-US is therefore often the shorter text.
- Units and numbers: `22.56 GiB` style (dot decimal separator, IEC units).
- Panel/island names are capitalised: "Drive & media", "Burning",
  "Disc image", "Maintenance", "Settings".
- Log lines start with a capital and end without a period; errors explain
  the cause and, where possible, give a concrete next step.

## Key translation decisions

- **"Formatteren" → "restore attempt"**: optical formatting is not a
  definitive wipe (like disk/SSD formatting) but a drive-dependent rescue
  operation. The en-US UI never promises erasure; the collapsible section
  explains that a failed attempt is risk-free for the data but may leave
  the disc temporarily unreadable until a power-cycle.
- **"Leeg" for BD-RE/DVD+RW**: libburn always declares overwritable media
  blank regardless of content; en-US texts must not claim the disc is empty
  in the data sense.
- **Multi-session Yes/No**: "Ja — schijf blijft open" /
  "Nee — schijf wordt afgesloten" became
  "Yes — disc stays open" / "No — disc gets closed".
