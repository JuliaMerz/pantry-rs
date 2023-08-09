use hyper;
use quick_error::quick_error;

use serde_json;

use std::convert::From;

quick_error! {
    #[derive(Debug)]
    pub enum PantryError {
        HyperError (err: hyper::Error) {
            display("hyper failure: {:?}", err)
            from()
        }
        Utf8Error (err: std::str::Utf8Error) {
            display("decoding failure : {:?}", err)
            from()
        }
        HyperHttpError (err: hyper::http::Error) {
            display("hyper http failure: {:?}", err)
            from()
        }
        DeserializationError (err: serde_json::Error) {
            display("Serde deseiralization failure: {:?}", err)
            from()
        }
        ApiError(status: hyper::StatusCode, msg: String) {
            display("API Returned {} — {}", status, msg)
        }
        OtherFailure(err: String) {
            display("Other Error: {:?}", err)
            from()
        }
    }
}
