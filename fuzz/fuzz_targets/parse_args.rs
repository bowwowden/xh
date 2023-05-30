#![no_main]
use libfuzzer_sys::fuzz_target;
use xh::download::total_for_content_range;


fuzz_target!(|input: (&str, u64)| {

    let (header, expected_start) = input;

    total_for_content_range(header, expected_start); 

});