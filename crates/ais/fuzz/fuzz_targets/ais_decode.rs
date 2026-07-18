// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use libfuzzer_sys::fuzz_target;
use tp_helm_ais::message::decode;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Decoding must never panic, even on malformed/malicious input.
        let _ = decode(s);
    }
});
