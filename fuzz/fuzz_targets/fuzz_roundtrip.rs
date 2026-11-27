#![no_main]

use libfuzzer_sys::fuzz_target;
use ozzyyaml::from_str;
use ozzyyaml_emit::to_string;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(value) = from_str(s) else {
        return;
    };
    let Ok(yaml) = to_string(&value) else {
        return;
    };
    if from_str(&yaml).is_err() {
        panic!("emit produced output the parser rejects: {yaml:?}");
    }
});
