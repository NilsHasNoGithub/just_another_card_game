#![allow(dead_code)]

extern crate rand;
extern crate serde;

#[macro_use]
extern crate derive_more;

pub mod data_structures;
pub mod socket_message_passing;

fn sub_vectors_are_same_length<T>(vector: &[Vec<T>]) -> bool {
    let mut len = None;
    for v in vector {
        match len {
            Some(l) => {
                if v.len() != l {
                    return false
                }
            }
            None => {
                len = Some(v.len())
            }
        }
    }
    true
}
