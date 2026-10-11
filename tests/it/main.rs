//! The integration tests, linked into one binary so the dependencies link once.

mod cli;
mod color;
mod progress;
mod wt_carry_lcov;

#[path = "../support/chain.rs"]
mod chain;
#[path = "../support/fixture.rs"]
mod fixture;
