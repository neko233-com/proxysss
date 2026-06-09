//! Integration verification modules (E2E + deep surface tests).

pub mod harness;

#[cfg(test)]
mod e2e;

#[cfg(test)]
mod deep;
