# AK820 Pro interoperability notes

Keyboard Studio was developed from offline USB analysis and operator-labelled experiments with the vendor configurator. The app's Rust source and drawings are original. These notes describe observed behavior and inference; no original Windows executable or UI assets are distributed.

## Transport

Only the observed wired device and matching HID interfaces are opened. Feature transfers use interface 3 with a leading zero report ID in hidraw, followed by 64 payload bytes. TFT output uses interface 2 and 4096-byte payload blocks. The 2.4GHz receiver has no supported write path.

Commands generally start with `04`, followed by an opcode. Replies must match the first three command bytes and have status byte 3 equal to `01`. Other bytes may contain additional data: `04 02 00 01 ff` is positive; `04 02 00 ff` is a rejection. Transfers stop on mismatched/rejected replies without automatic retries. An acknowledged commit followed by a failed refresh is reported separately.

Feature writes include 35 ms processing time. Map setup, begin and commit boundaries include an additional 35 ms; the final data block precedes commit by about 70 ms. Virtual-clock tests compare these gaps against captured timing. This corrects a former pacing discrepancy, but the cause of the operator's earlier intermittent rejection was not proven.

## Keymap tables

| Field | Meaning |
| --- | --- |
| `04 18` | Setup |
| `04 11`, byte 8 = 9 | Base (vendor Top) layer begin |
| `04 27`, byte 8 = 9 | Fn layer begin |
| Nine 64-byte blocks | 576-byte override table |
| Offsets 572–575 | `00 00 aa 55` marker/reserved bytes |
| `04 02`, then `04 f0` | Commit and refresh, each acknowledged |

Records occupy four bytes at `slot × 4`. A zero record delegates to the built-in default. Keyboard records use `02 00 usage 00`, modifiers use `02 modifier-bit 00 00`, and captured media/browser destinations use `03 usage-low usage-high 00`. The implementation allowlists supported records and source positions before opening a transaction.

The operator labelled 81 Base-layer physical positions. Controlled A→B and restoration independently establish A at slot 56. Controlled Fn I→Insert establishes Fn opcode `27`, I at slot 45 / offset 180 and Insert record `02 00 49 00`; the capture also has a zero-override table. Other Fn source positions reuse the Base layout as an inference. The operator subsequently tested and confirmed the implementation works.

Fn source restrictions follow the operator's driver hint: Esc, Backspace, Home, Win, Fn, Backslash/Pipe. Base still permits the other five positions; physical Fn is unsupported on both layers. Zeroing a layer restores its overrides, not every keyboard subsystem. Apply replaces a whole layer with the selected local profile, with no readback/merge.

The labelled Esc media sweep establishes 18 consumer actions: My Computer, Calculator, Mail, Multimedia, playback/track/volume, Mute and browser controls. Fixture tests compare 234 writes and 72 actual replies. The operator's action label “Volume” maps to captured usage `00ea`, displayed as Volume Down.

## Lighting and display

Two sweeps establish 111 lighting payloads, including twenty built-in modes. Mode IDs 1–19 are active effects; LED Off uses ID 0 and a zeroed body. RGB, colorful, brightness and speed fields are reproduced from the observed layout. Flowing has an observed byte-11 variant whose purpose is unknown; the normal UI uses zero.

Display upload fits the local 160×128 editor content to the observed 128×128 RGB565 format. Animation accepts 1–140 frames with per-frame delays (observed 2 ms units). Payloads use acknowledged 4096-byte output blocks. Clock sync uses the observed setup, data echo and finalize sequence. Full playback combinations, calendar edge cases and persistence are not exhaustively verified.

## Device settings and resets

Settings data carries response level at byte 8, sleep at byte 6, and Windows/Alt+F4/Alt+Tab locks at bytes 4/3/2. Twelve labelled transactions provide 48 writes and 36 replies. The user supplied timing descriptions for levels 1–5; they are vendor descriptions, not measurements by this project.

The reset capture SHA256 is `79c5ef780ad8948c8c2292c7e172ea490ca20f4b4cc8be96e5b3a77bd6e2376e`. Labels follow the filename/order listed: Reset Keyboard then Factory Reset. The operator subsequently tested the implementation and reported it works.

| Action | Workflow | Writes / replies |
| --- | --- | --- |
| Reset Keyboard | Clear Base and Fn override tables | 26 / 8 |
| Factory Reset | `0419`; eight-block `0415` clear; clear Base/Fn; captured lighting; captured settings | 46 / 17 |

The purpose of `0419` and the storage area selected by `0415` is not independently established. The implementation reproduces the complete observed workflow without labelling it as macro storage or claiming effects on uncaptured subsystems/TFT content. Factory defaults in the trace are lighting mode ID 11, RGB red, colorful 1, brightness 5, speed 3; settings Level 2, 5min sleep and all shortcut locks off. Factory lighting refresh is a write without a GET_FEATURE, unlike ordinary lighting Apply. Factory storage boundaries allow roughly 100 ms, reproduced with 105 ms in 35 ms units, within 6 ms of observed gaps.

Resets show their effect/target before sending, retain saved profiles/presets on the computer and stop with a partial-reset message if any stage fails. Reapplying a saved profile sends that profile's values again.

## Evidence and limits

`tests/fixtures/` retains control/configuration reports, actual command replies, capture hashes and operator annotations. `tools/captures/` validates each scoped write completion and reply window by bus/device; reused URB identifiers are paired chronologically. Raw captures and unrelated input traffic are excluded from this repository. Some older review metadata describes the evidence available at extraction time; later operator feedback is recorded above.

The regression suite covers byte-exact encoding, all command/read/write failure stages, rejection before transport, processing gaps, profile compatibility and selected UI behavior. Tests use fake transports and send no USB commands. User reports provide physical confirmation for the implemented workflow; exhaustive key combinations, other hardware/firmware revisions and power-cycle persistence remain open.
