use crate::core_iterator::*;
use crate::core_string::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Mailbox {
    pub capacity: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exit {
    pub reason: String,
}
