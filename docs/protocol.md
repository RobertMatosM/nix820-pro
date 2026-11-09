# AK820 Pro interoperability notes

Nix820 Pro was developed from offline USB analysis and operator-labelled experiments with the vendor configurator. The app's Rust source and drawings are original. These notes describe observed behavior and inference; no original Windows executable or UI assets are distributed.

## Transport

Only observed wired/receiver identities and matching HID interfaces are opened. Wired feature transfers use interface 3 with a leading zero report ID in hidraw, followed by 64 payload bytes. TFT output uses wired interface 2 and 4096-byte payload blocks.

Experimental receiver lighting uses `0c45:fdfd` interface 3 with an exact 32-byte input/output report descriptor. Each interrupt OUT message is `05 10 00`, the wired lighting encoder's first 16 bytes, 12 zero bytes and an additive checksum modulo 256. Interrupt IN must match the observed 32-byte reply including checksum and zero body. Linux writes include the leading zero report ID; input reports do not. See the [kernel hidraw API](https://docs.kernel.org/hid/hidraw.html) and [2.4 GHz investigation](wireless-investigation.md).

Receiver transfers clear queued input once before each transaction, use a two-second reply timeout and stop on malformed/unexpected replies or I/O errors without automatic retries. Unsupported receiver whole-layer/display jobs are rejected before opening a device. There is no sequence number in the observed lighting replies; drainage and transfer locking cannot distinguish a late response from another program that ignores the advisory lock. The operator tested wireless lighting and reported it working; failure semantics and persistence remain unverified. TFT transfers remain wired-only, consistent with the operator's observation that the original driver appears to intentionally omit wireless TFT support.

The follow-up wireless remap experiments establish single-key requests with Top opcode `03` or Fn opcode `0d`, byte 1 `04`, source slot at byte 2 and the wired-format four-byte destination record at bytes 3–6. Their details and acknowledgement limits are recorded in the [wireless investigation](wireless-investigation.md). The receiver runtime now sends one selected-key assignment, validates its layer/source before opening a node, and requires an exact zero-body acknowledgement with matching layer/source header and checksum. Base restoration resolves the physical key to its normal action. Fn defaults require an explicit destination: Use normal key assigns the printed key, rather than claiming to restore a factory Fn shortcut. Whole-layer replacement/defaults remain wired-only.

Wired commands generally start with `04`, followed by an opcode. Replies must match the first three command bytes and have status byte 3 equal to `01`. Other bytes may contain additional data: `04 02 00 01 ff` is positive; `04 02 00 ff` is a rejection. Transfers stop on mismatched/rejected replies without automatic retries. An acknowledged commit followed by a failed refresh is reported separately.

Feature writes include 35 ms processing time. Map setup, begin and commit boundaries include an additional 35 ms; the final data block precedes commit by about 70 ms. Virtual-clock tests compare these gaps against captured timing. This corrects a former pacing discrepancy, but the cause of the operator's earlier intermittent rejection was not proven.

A receiver remap clears queued input once, sends one request and waits under a fixed two-second deadline. It can skip the exact observed `20 01 00 ff` query response and the exact 32-byte `55 fc 04` notification with a zero body (the latter has a different format without the additive checksum). Neither is accepted as remap success. All other malformed, wrong-layer/source or nonzero-body replies fail; no more than eight input messages are processed. Timeouts and I/O errors after sending report an unconfirmed outcome without resending the command. The operator tested wireless Top/Base and Fn remapping and reported both working, without specifying the exact combinations. Other source positions and uncaptured media/modifier combinations reuse the established wired layout/records; exhaustive behavior and factory Fn restoration remain unverified.

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

Separate receiver captures establish wireless settings header `07 10 00`, the wired settings data's first 14 bytes plus `aa 55` at wireless bytes 17–18, zero padding and the usual additive checksum. Sixteen settings actions have matching acknowledgements. Wireless Reset Keyboard uses `1f` and Factory Reset uses `0f`; both have captured acknowledgements, but `1f` replies after 1.336 seconds with two unrelated `20 01 00` queries in between. These commands do not imply the wired reset workflows or effects. See the [wireless settings/reset investigation](wireless-investigation.md). These features are enabled in the receiver runtime; the operator tested them and confirmed they work. Reset Keyboard sends the captured follow-up queries at approximately 351 ms and 1272 ms without clearing intervening input, then waits under one two-second deadline. Only its exact `1f` reply confirms the reset; up to two captured `20 01 00 ff` query replies can be skipped. Factory Reset sends only `0f` and checks its own reply. Neither workflow retries the reset on failure.

| Action | Workflow | Writes / replies |
| --- | --- | --- |
| Reset Keyboard | Clear Base and Fn override tables | 26 / 8 |
| Factory Reset | `0419`; eight-block `0415` clear; clear Base/Fn; captured lighting; captured settings | 46 / 17 |

The purpose of `0419` and the storage area selected by `0415` is not independently established. The implementation reproduces the complete observed workflow without labelling it as macro storage or claiming effects on uncaptured subsystems/TFT content. Factory defaults in the trace are lighting mode ID 11, RGB red, colorful 1, brightness 5, speed 3; settings Level 2, 5min sleep and all shortcut locks off. Factory lighting refresh is a write without a GET_FEATURE, unlike ordinary lighting Apply. Factory storage boundaries allow roughly 100 ms, reproduced with 105 ms in 35 ms units, within 6 ms of observed gaps.

Resets show their effect/target before sending, retain saved profiles/presets on the computer and stop with a partial-reset message if any stage fails. Reapplying a saved profile sends that profile's values again.

## Evidence and limits

`tests/fixtures/` retains control/configuration reports, actual command replies, capture hashes and operator annotations. `tools/captures/` validates each scoped write completion and reply window by bus/device; reused URB identifiers are paired chronologically. Raw captures and unrelated input traffic are excluded from this repository. Some older review metadata describes the evidence available at extraction time; later operator feedback is recorded above.

The regression suite covers byte-exact encoding, all command/read/write failure stages, rejection before transport, processing gaps, profile compatibility and selected UI behavior. Tests use fake transports and send no USB commands. User reports provide physical confirmation for the implemented workflow; exhaustive key combinations, other hardware/firmware revisions and power-cycle persistence remain open.
