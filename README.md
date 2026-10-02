<!-- wisent-banner:start -->
<p align="center">
  <img src="assets/readme-banner.webp" alt="email-sequencing-cli by Wisent" width="100%">
</p>
<!-- wisent-banner:end -->

<!-- wisent-readme-signals:start -->
[![Source](https://img.shields.io/badge/GitHub-Source-181717?logo=github)](https://github.com/wisent-ai/email-sequencing-cli) [![Issues](https://img.shields.io/badge/GitHub-Issues-181717?logo=github)](https://github.com/wisent-ai/email-sequencing-cli/issues) [![Wisent](https://img.shields.io/badge/Wisent-Website-0B0B0B)](https://wisent.com) [![Discord](https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white)](https://discord.gg/qRjpkthq54) [![LinkedIn](https://img.shields.io/badge/LinkedIn-Follow-0A66C2?logo=linkedin&logoColor=white)](https://www.linkedin.com/company/wisent-ai/) [![X](https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white)](https://x.com/wisentai) [![Enterprise](https://img.shields.io/badge/Enterprise-Book%20a%20call-0B0B0B?logo=calendly)](https://calendly.com/lbartoszcze)
<!-- wisent-readme-signals:end -->

# email-sequencing-cli

Local planning and execution state for email sequences. It never sends
email. Every command reads JSON files and prints JSON, or with `--text` one
`path: value` line per field from the same document.

| Command | What it prints |
|---|---|
| `email-sequencing enroll --sequence <sequence.json> --enrollment <seed.json> [--at <RFC 3339>]` | the new enrollment; `--at`, else the seed's `enrolledAt`, else now |
| `email-sequencing schedule --sequence <sequence.json> --enrollment <enrollment.json>` | every step of one enrollment with the time it falls due |
| `email-sequencing plan --sequence <sequence.json> --enrollments <enrollments.json> [--at <RFC 3339>]` | the steps due across many enrollments at `--at` (now when omitted) |
| `email-sequencing record --enrollment <enrollment.json> --event <event.json>` | the enrollment with the event added (`sent`, `skipped`, `replied`, `bounced`, `unsubscribed`) |
| `email-sequencing render --template <template.txt> --variables <variables.json>` | the template with its variables filled |

`--help` works on the program and on every command. Exit 2: the invocation
is wrong (unknown command or flag, missing argument); exit 1: an input file
could not be read, is not valid JSON, or was refused, with the reason on
stderr.
