# PCAP Artifacts

These packet captures are preserved as high-value object-ingress reference artifacts.

Do not overwrite or rename these files casually. If a newer capture supersedes them, keep the old
artifact and add a new dated file plus a note explaining the relationship.

## Preserved Captures

- `firestorm_object_ingress_reference_2026-03-30_firee.pcapng`
  - source: `C:\Users\matti\Desktop\firee.pcapng`
  - sha256: `BA4EE224D69F30244F0700AF69BD03C1EE6D4E21F72C7B55C70CAD0AF43DF91B`
  - role: working Firestorm reference showing the pre-burst startup sequence and first `ObjectUpdateCached` burst on `16.144.39.130:13001`

- `app_object_ingress_reference_2026-03-30_App.pcapng`
  - source: `C:\Users\matti\Desktop\App.pcapng`
  - sha256: `C870823C1A0563D2E83D2B0C7F8155C61CD44E931CA5C0D972BE47DE027EE9A1`
  - role: captured app reference showing the blocked startup path and the additional local-port evidence on the same simulator endpoint

## Why They Matter

- The Firestorm capture confirms the exact viewer-to-simulator sequence immediately before the first object burst.
- The app capture shows that the blocked viewer path reaches the same simulator endpoint but does not receive the matching object burst.
- Together they are the current best external runtime evidence for the object-ingress investigation.
