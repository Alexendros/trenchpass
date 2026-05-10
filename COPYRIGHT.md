# Copyright

TrenchPass · MCP gateway custodio único de credenciales
Copyright © 2026  Alexendros (`spiderwebtraveler@gmail.com`)

This program is free software: you can redistribute it and/or modify it under
the terms of the **GNU Affero General Public License v3.0 or any later version**
as published by the Free Software Foundation.

This program is distributed in the hope that it will be useful, but **WITHOUT
ANY WARRANTY**; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
details.

You should have received a copy of the GNU Affero General Public License along
with this program. If not, see <https://www.gnu.org/licenses/agpl-3.0.html>.

## SPDX

```
SPDX-FileCopyrightText: 2026 Alexendros <spiderwebtraveler@gmail.com>
SPDX-License-Identifier: AGPL-3.0-or-later
```

## Third-party notices

TrenchPass links against the following third-party crates whose licenses must be
preserved when redistributing binaries. The exhaustive list — kept in sync by
`cargo about generate` — lives at `THIRD_PARTY_NOTICES.md` (regenerated each
release). Highlights:

| Crate                       | License                       |
| --------------------------- | ----------------------------- |
| `rmcp`                      | Apache-2.0                    |
| `tokio` / `tracing` family  | MIT                           |
| `axum` / `tower-http`       | MIT                           |
| `rustls` / `tokio-rustls`   | Apache-2.0 OR MIT OR ISC      |
| `vaultrs`                   | MIT                           |
| `sqlx`                      | Apache-2.0 OR MIT             |
| `opentelemetry*`            | Apache-2.0                    |
| `governor`                  | MIT                           |
| `serde` / `serde_json`      | Apache-2.0 OR MIT             |
| `x509-parser`               | MIT OR Apache-2.0             |

The vendored audit schema in `sql/init_audit.sql` is co-licensed AGPL-3.0
because it is a derivative of the schema authored for Controlink under the
same terms.

## Network-service distribution clause (AGPL §13)

Because TrenchPass is **operated as a network service** by Alexendros, any
modified version exposed to remote users (whether self-hosted or SaaS)
**must offer the corresponding source code** to those users. The canonical
mechanism is the `/source` HTTP endpoint that the gateway will expose in PR9
linking to the public Forgejo mirror.

## Trademark

"TrenchPass" and "Alexendros" are unregistered trademarks of the copyright
holder. Use of these names in derivative works to imply endorsement is not
permitted.
