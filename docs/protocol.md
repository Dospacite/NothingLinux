# Independently implemented B171 protocol

Nothing Linux was implemented from observed device traffic, the public Bluetooth service record, and clean-room behavioral testing. No Nothing X code or resources are included.

## Transport and frame

The control service UUID is `aeac4a03-dff5-498f-843a-34487cf133eb`; B171 exposes its control stream on RFCOMM channel 15.

After the nonblocking socket connect completes, the client waits for `getpeername()` to confirm that the RFCOMM data-link connection is attached before sending the handshake. If that check still returns `ENOTCONN`, the app performs a bounded, rate-limited BlueZ disconnect/reconnect cycle and retries instead of treating the stale socket as a live control session.

Every host frame is:

```text
55 | control:u16-le | command:u16-le | length:u16-le | sequence:u8 | payload | crc:u16-le
```

Host control is `0x0160`. CRC16-ARC covers the complete frame before the CRC, starts at `0xffff`, and uses reflected polynomial `0xa001`. Payloads larger than 4096 bytes are rejected before allocation. Responses clear command bit 15, while unsolicited events use the `0xe0xx` range.

The session queries protocol version (`c001`), activates (`f001`), reads remote configuration (`c006`), and only enables writes when the returned serial SKU maps to B171. Confirmed B171 SKUs are 61, 62, 69, 70, 74, and 75.

## Confirmed commands

| Function | Read | Write/event |
|---|---:|---:|
| Battery | `c007` | `e001` |
| Earbud/wear status | `c00a` | `e002` |
| In-ear detection | `c00e` | `f004` |
| Gestures | `c018` | `f003` |
| ANC | `c01e` | `f00f` / `e003` |
| EQ preset | `c01f` | `f010` |
| High-quality audio codec | `c029` | `f01c` |
| Dual connection | `c027` | `f01a` |
| Low-lag mode | `c041` | `f040` |
| Firmware | `c042` | read-only |
| Three-band custom EQ | `c044` | `f041` |
| Advanced EQ enabled | `c04c` | `f04f` |
| Advanced EQ profile | `c04d` | `f050` |
| Bass Enhance | `c04e` | `f051` |
| Find earbud | — | `f002` |
| Ear-tip fit test | — | `f014` / `e00d` |

Writes are serialized. The UI retains the last confirmed value until an acknowledgement and query readback arrive. A missing acknowledgement produces a failure event after four seconds. Unknown models and unverified feature commands are rejected before framing.

EQ preset writes use one-byte mode values: balanced `00`, voice `01`, more treble `02`, more bass `03`, and simple custom EQ `05`. High-quality audio codec selection uses `f01c/c029` with default `00`, LHDC `01`, and LDAC `02`.

Raw protocol logging is off by default. Normal diagnostics redact Bluetooth addresses and serial-like identifiers.
