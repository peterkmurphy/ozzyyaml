#![no_main]

use libfuzzer_sys::fuzz_target;
use ozzyyaml_parser::parse_str;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let _ = parse_str(s);
});
