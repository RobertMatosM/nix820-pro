# 2.4 GHz protocol investigation

Source: `tests/wireshark/usb-wireless-light-modes.pcapng`, captured 2026-11-07.
SHA256: `d10e7e766647712900a430095ced272826b4f112b4c68612e354c1b53fb86585`.
The operator selected Static through LED Off in order. Analysis was read-only.
Experimental lighting support is implemented on `dev`; the operator tested it
and reported it working. This confirmation does not establish every setting
combination, persistence or keyboard-off behavior.

## Observed transport

Device descriptor frame 102 identifies `0c45:fdfd`, revision `0138`.
Configuration descriptor frame 110 exposes interface 3 with 32-byte interrupt
OUT `05` / IN `85` endpoints. Interface 4 has separate 64-byte interrupt
endpoints `06` / `86`, but there are no transfers on them in this capture.

The connected receiver's sysfs descriptors independently match that identity
and endpoint layout. Interface 3's HID report descriptor is:

```text
0660ff0961a1010962150026ff009520750881020963150026ff00952075089102c0
```

It declares unnumbered 32-byte input/output reports. The leading `05` in a
lighting packet is part of its protocol header, not a HID report ID. A Linux
hidraw output write includes the leading zero report-ID byte, as specified by
the [kernel hidraw API](https://docs.kernel.org/hid/hidraw.html).

## Captured messages

There are 22 interrupt OUT submissions, each with a successful USB completion,
and 22 following IN replies with matching three-byte headers. Twenty pairs are
lighting; two use `20 01 00` and have an unknown purpose. The first IN reply's
read submission predates the capture; the remaining 21 reads have captured
submission/completion pairs. URB IDs are reused and were matched chronologically.

Lighting requests have this 32-byte layout (zero-based offsets):

- Bytes 0–2: `05 10 00`.
- Bytes 3–18: exactly bytes 0–15 of the existing wired lighting encoder.
- Bytes 19–30: zero in this capture.
- Byte 31: sum of bytes 0–30 modulo 256.

Mode IDs are exactly `1` through `19`, then `0` for LED Off, matching the wired
mapping. Active modes use red `ff 00 00`, colorful `1`, brightness `5`, speed
`3` and variant `0`. LED Off zeros the body before the `aa 55` marker.
All 20 requests match that wired representation byte for byte. The follow-up
captures below extend the evidence for colors, colorful, brightness and speed.
Flowing's variant remains untested wirelessly.

Every lighting reply is `05 10 00`, 28 zero bytes, then checksum `15`.
The additive checksum rule holds for all 44 request/reply packets, including
the two unknown transactions. Failure replies and their status semantics have
not been captured. A successful USB completion or matching receiver reply alone
does not establish the physical keyboard effect or persistence.

The first lighting request/reply is frames 3334/3627; LED Off is 16597/16611.
First lighting reply latency is 454.352 ms. The remaining 19 are 22.369–27.758 ms;
the median across all 20 is 23.986 ms. The reason for the initial delay is unknown.
No wired setup/begin/commit/refresh transaction accompanies these commands.
The control requests in this trace are ordinary descriptor reads.

## Static RGB/colorful/brightness follow-up

Source: `tests/wireshark/usb-wireless-static-rgb-colorful-rgb-brightness.pcapng`.
SHA256: `e8eaa46e7c859c6f6f4e27b19dc7fd45d05a50ac8fe73947b3fb771b167b44b7`.
The operator varied RGB, colorful and brightness in Static mode without a fixed
order. Despite that, consecutive lighting requests include isolated changes for
each field, allowing the byte positions to be distinguished.

There are 25 successful OUT submissions/completions and 25 following replies:
22 Static lighting pairs (13 distinct lighting requests) and three unknown
`20 01 00` pairs. All 50 packets obey the same 32-byte/checksum format. The first
IN read submission predates this capture; the remaining 24 reads are paired.
Every lighting body matches the existing wired encoder's first 16 bytes, with
the same zero padding and unchanged lighting reply as the mode sweep.

- RGB, at wireless bytes 4–6: red, orange, yellow, magenta, white, black and cyan.
  Frames 11621→13151 and 13151→14045 are examples with only RGB/checksum changing.
- Colorful, at byte 11: both 0 and 1. Frames 6631→7933 change only this field
  and checksum, holding orange RGB, brightness 5 and speed 3 constant.
- Brightness, at byte 12: levels 1, 3, 4 and 5. Frames 3846→5779→6631 change
  only brightness/checksum, with orange RGB and colorful disabled.
- Speed, at byte 13: always 3. Mode is always 1 (Static).

Lighting reply latency is 21.661–26.193 ms, median 23.713 ms. No additional
setup, commit or refresh messages accompany these changes. Repeated identical
requests are ordinary separate transfers in the trace, not evidence that the
implementation must retry each command.

This is sufficient evidence to implement the observed RGB/colorful/brightness
fields without another tidy Static capture. Brightness 2 is not directly
observed; its encoding is inferred from the other levels and wired behavior.
Physical output, failure replies and persistence are still separate checks.

## SingleOn speed follow-up

Source: `tests/wireshark/usb-wireless-singleon-light-speed.pcapng`.
SHA256: `8324dbfa58d6e319912553e9ab682e31b96e994b9bbdb724a1077a368cec770d`.
Five lighting requests select speed 1, 2, 3, 4 and 5 in order, with mode ID 2
(SingleOn), red RGB, colorful 1 and brightness 5 held constant. Only wireless
byte 13 and checksum byte 31 change between consecutive requests.

All five requests have successful USB completions and following matching replies;
all ten checksums validate. Bodies match the wired encoder's first 16 bytes
exactly. The first read submission predates the capture; the other four reads
are paired. The replies have the same contents as in the preceding captures,
with latencies 21.709–24.763 ms. First/last request frames are 2744/6113.

Speed levels now have complete direct coverage in one animated mode. Together,
the three captures establish the encoding for all normal lighting controls.
They do not measure physical animation rates or prove every cross-mode setting
combination, failure behavior or persistence.

## Top and Fn remap follow-up

The operator captured Top A→B→default and Fn I→Insert→default. Capture hashes:

- `usb-wireless-remap-top-layer-atob-default.pcapng`:
  `aadc33a1f7a1855d51ce45639a5850b940a7c334f52d2f49802f1399d1b8243c`.
- `usb-wireless-remap-fn-layer-itoinsert-default.pcapng`:
  `5114f0eafed94f87636816667befbac8f6e6a4a7ab45cf0fd83db97492822b98`.

Each operation is one 32-byte interrupt OUT request on the same receiver
interface. No wired setup/begin/576-byte table/commit workflow appears.
The observed request layout is:

- Byte 0: `03` for Top, `0d` for Fn.
- Byte 1: `04`, consistent with a four-byte destination record; the length
  interpretation is inferred from these experiments.
- Byte 2: source slot, matching the wired layout: A=`38` (56), I=`2d` (45).
- Bytes 3–6: the same four-byte destination format used by the wired encoder.
- Bytes 7–30: zero; byte 31: additive checksum modulo 256.

Top frame 7115 writes A→B with record `02 00 05 00`; frame 12931 restores
original A with `02 00 04 00`. Fn frame 5217 writes I→Insert with
`02 00 49 00`; frame 9209 restores original I with `02 00 0c 00`.
All four USB write completions succeed and all request checksums validate.

Matching remap replies echo the first three request bytes, zero bytes 3–30 and
carry the same additive checksum rule. A→B and both Fn operations have these
replies, at 23.152, 22.721 and 21.753 ms respectively. Each matching read's
submission and completion are captured.

Top A restoration has **no matching remap acknowledgement**. Its later input,
frame 15365, starts `55 fc 04` and is not a valid acknowledgement in the observed
format. Its purpose is unknown. The operator explicitly confirmed that A returned
to normal and the original driver showed no error. That is physical outcome
evidence, rather than a replacement for the missing protocol acknowledgement.
The unrelated initial `20 01 00` exchanges are excluded from remap fixtures.

Restoration writes an explicit original-key destination, not a zero record.
These captures therefore establish per-key operations, not a whole-layer clear
or a safe way to replay all zero/default slots. The runtime now applies selected keys and describes restoration explicitly. A whole-layer
operation would need defined defaults and partial-failure handling; it cannot
reuse the wired table transaction. Other source slots and media/modifier records
still rely on inference until tested wirelessly.

`tests/fixtures/wireless-remap-review.json` retains the four scoped requests,
three actual matching replies, the explicit acknowledgement gap and the operator
confirmation. `tools/captures/wireless_remap_capture.py` reproduces that review:

```sh
python3 tools/captures/wireless_remap_capture.py tests/wireshark/usb-wireless-remap-top-layer-atob-default.pcapng --layer Top --bus 1 --device 4
python3 tools/captures/wireless_remap_capture.py tests/wireshark/usb-wireless-remap-fn-layer-itoinsert-default.pcapng --layer Fn --bus 1 --device 4
```

The receiver runtime supports selected-key remapping, lighting, device settings and resets. Whole-layer keymap operations remain wired-only.

## System Settings and reset follow-up

Five operator-labelled captures cover settings and both reset buttons:

- `usb-wireless-systemsettings-levels-1to5.pcapng`: response levels 1–5,
  with sleep 5min and all locks disabled.
- `usb-wireless-systemsettings-sleeptime-offto30min.pcapng`: No sleep
  (twice), 1min, 5min, 30min, with response level 2 and all locks disabled.
- `usb-wireless-systemsettings-sleeptime-disable-all.pcapng`: Windows lock,
  then Alt+F4, then Alt+Tab enabled cumulatively, then disabled in reverse order,
  with response level 2 and sleep 1min.
- `usb-wireless-systemsettings-resetkeyboard.pcapng`: one Reset Keyboard.
- `usb-wireless-systemsettings-factoryreset.pcapng`: one Factory Reset.

The 16 ordinary settings actions each use one 32-byte request:

- Header: `07 10 00`.
- Bytes 3–16: the wired settings encoder's first 14 bytes.
- Bytes 17–18: `aa 55`, relocated from wired bytes 62–63.
- Bytes 19–30: zero. Byte 31: sum of bytes 0–30 modulo 256.

Absolute wireless field positions are response level at byte 11, sleep at byte
9, Windows lock at byte 7, Alt+F4 lock at byte 6 and Alt+Tab lock at byte 5.
This preserves the wired field layout inside the 16-byte body, with a different
marker position. All five level values, all four sleep values and both states
of each lock are directly captured. There is no wired setup/data/commit sequence.

Each settings acknowledgement is `07 10 00`, 28 zero bytes, checksum `17`.
All 16 settings writes complete successfully and have matching acknowledgements,
at 15.324–19.683 ms. Packet encoding does not measure actual response latency or
establish that every physical setting took effect.

The resets are distinct receiver commands, not the wired multi-stage workflows:

- Reset Keyboard: byte 0 is `1f`, bytes 1–30 zero, checksum `1f`.
  Frame 2057 is acknowledged by frame 3808 after **1335.558 ms**. Two other
  `20 01 00` requests occur in between, at 351.171 and 1271.780 ms after reset.
  Those writes succeed, but no separate `20 01 00` replies are captured during
  this reset. Their exact purpose and whether they are necessary to obtain the
  reset acknowledgement are not established. They must not be mistaken for
  additional resets or have the delayed `1f` response attributed to them.
- Factory Reset: byte 0 is `0f`, bytes 1–30 zero, checksum `0f`.
  Frames 2805/2813 form a request/acknowledgement pair at **11.577 ms**.
  A later `20 01 00` request/reply is separate from this acknowledged reset.

Both reset replies exactly match their requests. All four reset request/reply
checksums validate. The captures establish the labelled commands and observed
sequence, not the full effect on remaps, lighting, TFT content, pairing or
persistent storage. The runtime's wired reset behavior must not be used as proof
of the wireless commands' effects.

[Scoped settings/reset fixtures](../tests/fixtures/wireless-settings-review.json)
retain all 18 action requests/replies, capture hashes, measured response gaps and
the two intervening queries in the Reset Keyboard trace. Unrelated input traffic
is excluded. Reproduce a review with:

```sh
python3 tools/captures/wireless_settings_capture.py tests/wireshark/usb-wireless-systemsettings-levels-1to5.pcapng --kind levels --bus 1 --device 4
```

Other `--kind` choices are `sleep`, `locks`, `reset-keyboard`, `factory-reset`.
Ordinary settings and both resets are now implemented. Reset Keyboard sends its
captured `1f` request, waits 351 ms, sends the captured `20 01 00` query, waits
921 ms and sends the second query. Input is cleared only before the transaction,
so an early reset acknowledgement stays queued. A single two-second deadline
then bounds reply processing: only the exact reset acknowledgement succeeds;
up to two exact captured query replies may be skipped. No reset is resent on
failure. Factory Reset sends only its acknowledged `0f` command; the later query
in that trace is outside its completed reset transaction.

Settings data is validated against the canonical wired encoder before any device
is opened. Receiver reset confirmation describes the receiver operation rather
than claiming the wired multi-stage reset's exact effects. The transfer lock is
held throughout waits and reply processing. Errors after starting reset explicitly
report that its outcome is unconfirmed and it may already have taken effect.
The operator subsequently tested wireless settings and resets and confirmed they
work. No real reset was sent by the agent during implementation. This feedback
does not establish exhaustive combinations, every reset side effect or persistence.

## Implementation and validation

Implemented components:

1. Discovery by VID/PID, interface and exact report descriptor, with explicit
   wired/receiver labels and identity revalidation before opening a node.
2. Separate 32-byte interrupt request/reply handling that reuses the wired
   lighting encoder's first 16 bytes and adds the observed wrapper/checksum.
   A two-second timeout accommodates the first delay observed in the mode sweep.
3. Selected-key remap, lighting, settings and reset routing by connection type.
   Whole-layer keymap/display receiver jobs are blocked before opening a device
   and their UI transfer controls are disabled. Transfer locking and bounded stale-input drainage
   precede each transaction; related reset writes never drain a pending reply.
4. Scoped fixtures for all 47 lighting pairs and 18 settings/reset actions,
   Rust encoding/exchange and virtual-clock reset tests, Linux socket-based I/O
   tests and UI coverage of enabled settings/resets and unavailable actions.
   The Python extractor validates write completions, reply windows, checksums
   and source scope; its tests need no tshark or keyboard.

Re-extract a capture with:

```sh
python3 tools/captures/wireless_capture.py tests/wireshark/CAPTURE.pcapng --bus 1 --device 4
```

Output goes to ignored `capture-output/`. Only lighting configuration traffic is
included in the tracked fixture; unrelated input traffic and the unknown
`20 01 00` exchanges are excluded. Read submission boundaries that predate the
capture are marked explicitly.

No further normal lighting sweep is needed before implementation. Brightness 2
remains inferred; it can be verified during physical testing. Test a first
change after sleep and keyboard-off/unavailable behavior to distinguish receiver
acknowledgement from keyboard delivery. These are robustness checks rather than
reasons to repeat the normal controls captures.

The operator reports that TFT appears intentionally unavailable in the original
driver's wireless mode. TFT transfers, including clock sync, remain wired-only;
the unused 64-byte interface does not establish TFT capability or a reason to
implement wireless display writes. This is a product/UI observation rather than
proof that the hardware could never support them.

Selected-key wireless remapping is now implemented after the System Settings work. The new settings/reset captures establish those
commands as described above. The modifier/media follow-up below establishes
outgoing record formats; physical acceptance still needs testing.

## Remaining remap evidence

Selected-key keyboard assignments on Top/Fn are implemented from the existing
captures, reusing the wired source layout as an inference outside A/I. No complete
81-position wireless sweep is required before starting that implementation.

The targeted experiments requested were:

- Top A→Left Ctrl→Calculator→default: this exercises modifier-bit records and a
  consumer record with a nonzero high usage byte. The existing wired encodings
  establish the candidate bytes. The new follow-up below captures those writes,
  with no receiver input or physical confirmation.
- Choose a remappable Fn-layer key with a known built-in secondary action;
  remap it to B, restore it with the original driver, and confirm the original
  Fn action returns. Fn I→default establishes only the explicit I destination,
  not restoration of other built-in Fn actions. The follow-up again uses I and
  does not establish restoration of a factory Fn shortcut.

The wireless UI applies only the selected key. Whole-layer replacement
and whole-layer defaults need separate behavior: a zero local override delegates
to built-in defaults in the wired table but is not an observed receiver remap
record. Do not translate every zero slot into an original-key assignment or use
the broader Reset Keyboard action as a layer-specific restore.

## Modifier/media and Fn replacement follow-up

Two new captures and their hashes:

- `usb-wireless-remap-top-layer-atolctrltocalculatortodefault.pcapng`:
  `a6ef9499f5ad15361d61b98caf9cf142f85f6335bca92958778cd8b1bb74fc42`.
- `usb-wireless-remap-fn-layer-itoinserttobtodefault.pcapng`:
  `89874ff8603b55e2150b3cedc4025f410b41a8df4b88d31c20265135460d8db2`.

Top A (slot 56) receives three requests:

- Frame 4697: Left Ctrl, record `02 01 00 00`.
- Frame 7429: Calculator, record `03 92 01 00` (consumer usage `0192`).
- Frame 12823: original A, record `02 00 04 00`.

All requests have valid checksums and successful USB completions. Modifier-bit
and two-byte consumer records exactly match the existing wired encodings.
However, this entire Top capture contains **no endpoint 85 submissions or
completions**, for either remaps or the other receiver commands. There are no
device-level remap acknowledgements. The operator did not test the physical key.
These are confirmed driver-generated writes, not verified receiver acceptance;
the absence of input does not establish rejection or its cause.

The Fn capture uses I (slot 45) throughout, following the operator's stated
I→Insert step before the remaining changes:

- Frame 2477: Insert, record `02 00 49 00`; reply 2491 after 24.057 ms.
- Frame 8297: B, record `02 00 05 00`; reply 8423 after 238.164 ms.
- Frame 12939: original I, record `02 00 0c 00`; reply 12955 after 23.960 ms.

All three have successful USB completions and matching, checksum-valid remap
acknowledgements with captured read submission/completion pairs. This establishes
replacement of an existing custom Fn assignment and an explicit I restoration.
The operator is unsure what Fn+I did before the experiment or after Default.
Assigning Insert during the experiment does not establish a factory Fn action;
its restoration remains untested for keys that have built-in secondary actions.

### Extra receiver traffic

Each new capture has four host-issued `20 01 00` queries on OUT endpoint 05,
separate from the remaps. The Fn capture has four corresponding-format IN
messages (`20 01 00 ff`, zero padding, checksum `20`); Top has none. All eight
query writes have successful USB completions and valid checksums. Query replies
can arrive after more than one identical query was sent; without a sequence
number, individual request/reply attribution is uncertain.

This traffic is initiated by the host, rather than an unsolicited dongle
handshake. Its presence across lighting, settings, resets and remaps is consistent
with background/status polling, but its exact purpose is unknown. In particular,
the `ff` data byte is not established as a wired-style rejection status or a
battery value. Our normal lighting/settings paths work without ongoing queries;
no general handshake requirement has been demonstrated. Reset Keyboard retains
the separate two-query sequence from its own capture.

The earlier `55 fc 04` input is another, still-unidentified message format. It
must not be accepted as a remap acknowledgement. Other devices' traffic also
appears in these bus-wide captures and is excluded from the scoped evidence.

The remap extractor now bounds each reply window by the next request with the
same layer/source header, rather than any outgoing message. This permits a
delayed remap acknowledgement across unrelated queries without treating those
query replies as remap success. Scoped evidence is stored in
`tests/fixtures/wireless-remap-followup-review.json`; reproduction uses the
existing extractor with `--sequence followup` and the appropriate `--layer`.

Selected-key assignments are implemented from this evidence. Modifier/media
effects still need physical testing. Factory Fn restoration and
whole-layer defaults must remain distinct from assigning a physical key's normal
letter/action; the new captures do not establish a general factory-default clear.

## Selected-key remapping implementation

The receiver uses a distinct selected-key job; the wired full-layer table job is
still rejected for receivers. Before opening any node, the encoder checks the
source against the existing physical layout and layer restrictions. All typed
keyboard/modifier/media/browser destinations use their existing four-byte records.
Ten captured requests match the encoder exactly; six have actual matching replies.
The four acknowledgement gaps remain explicit and are never promoted to success.

Apply selected key resolves only the selected profile entry, including an imported
captured-table entry when present. Unrelated legacy overrides are not sent and do
not block that selected-key transaction. An ambiguous legacy label on the selected
key must be reselected before Apply.

- Base Restore key saves a local default; Apply sends that physical key's ordinary
  keycode. Modifier sides and shifted keycap labels have explicit physical-key
  translations, independent of the old ambiguous destination-label importer.
- Fn Use normal key saves an explicit action for the printed key. It does not
  clear a firmware override or restore a built-in Fn shortcut. An unset Fn entry
  cannot be sent; choose a destination or use this explicit assignment.
- Receiver whole-layer defaults are disabled. Wired Apply/Restore keep their
  existing full-layer behavior. Local editing never sends a command itself.

Each remap transaction drains stale input once, sends one message, then accepts
only the exact layer/source acknowledgement with valid checksum and zero body.
Within one two-second deadline, up to eight input frames may be inspected. Only
the exact previously captured `20 01 00 ff` response and zero-body `55 fc 04`
notification may be skipped. The notification follows its own observed format;
it is not checksum-validated as an ordinary remap packet. Unknown messages and
bad remap acknowledgements stop the transfer. Errors after sending explicitly
report an unconfirmed outcome; there are no automatic retries or handshake queries.

Tests cover all ten captured writes and six actual replies, acknowledgement gaps,
background interleaving, a fixed deadline and message-count bound, wrong
layer/source/status/checksum/length, drain/write/read failure, source restrictions,
physical-key translation, selected-key profile resolution and UI action routing.
The operator subsequently tested the implementation and reported that both
wireless Top/Base and Fn remapping work. The exact key/destination combinations
were not specified; this does not establish every modifier/media action, source
position or factory Fn restoration. No hardware remap was sent by the agent
during implementation.
