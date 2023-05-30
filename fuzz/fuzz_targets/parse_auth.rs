#![no_main]
use libfuzzer_sys::fuzz_target;
use xh::test::server::Server;

fuzz_target!(|input: &str| {

    // let server = server::http(|req| async move {
    //     // assert_eq!(req.method(), "GET");
    //     hyper::Response::builder().body(input.into()).unwrap()
    // });

    // get_command()
    //     .args(["--print=b", "get", &server.base_url()])
    //     .assert()
    //     .stdout(input);
    
});