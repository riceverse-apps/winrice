# Security Policy

## Reporting a vulnerability

Please **do not open a public issue** for security problems. Use GitHub's private vulnerability reporting: repository → *Security* → *Report a vulnerability*. Include what you found, how to reproduce it, and the impact you think it has.

You will get an answer as soon as a maintainer can look at it. This is a volunteer project; there is no paid bounty.

## What we consider in scope

- A theme or catalog entry that can run code, write outside its folder, or read files it should not (path traversal, forbidden file types, size limits).
- Commands from imported themes being executed. Themes never carry commands; only official tool manifests do.
- Downloads accepted without checking the SHA-256 listed in `catalog/index.json`, or from hosts other than GitHub.
- A way to make WinRice change the system without going through the Transaction Engine (no backup / no rollback).
- Data leaving the computer. WinRice has no telemetry and none must be added.
- A way to make the elevated administrator helper change anything other than the scheduled tasks named by official tool manifests.

## Design rules that protect users

- The app runs without administrator rights and asks for elevation only for the steps that need it, through the Windows prompt. The elevated copy of the app accepts a single command (change a scheduled task) and only for tasks named in an official tool manifest.
- Imported themes are shown as *unverified* with the full plan before anything is applied.
- Files are backed up before being changed; the first original of each file is kept for "Restore original Windows".
- The only links the app opens belong to this project's repository.

## Known limitations

- The project has no code-signing certificate yet, so Windows SmartScreen may warn on the installer.
- Third-party tools are installed from their official sources; their security is their authors' responsibility.
