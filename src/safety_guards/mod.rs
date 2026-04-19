#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreRequirement {
    NotRequired,
    Required,
}
