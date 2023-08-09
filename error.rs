use diesel;
use hyper;
use quick_error::quick_error;
use tiny_tokio_actor::ActorError;

use std::convert::From;

quick_error! {
    #[derive(Debug)]
    pub enum PantryError {
        HyperError (err: hyper::Error) {
            display("ActorError failure: {:?}", err)
            from()
        }
        OtherFailure(err: String) {
            display("Other Error: {:?}", err)
            from()
        }
    }
}
