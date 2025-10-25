# Simple Rust Cov

This is a basic CLI utility that automates the process of reporting unit test coverage for a Cargo project. It allows for setting a threshold on line and branch coverage requirements (which both default to 100%), and prints out coverage in the standard rust llvm-cov format.

## Installing

### Prerequisites

To install the necessary llvm tools, run:

```bash
rustup component add llvm-tools
```

And then, if you havn't already, install `cargo-binutils` with:

```bash
cargo install cargo-binutils
```

This will add the previously installed llvm-tools to your PATH.

### Cargo Install

To install this CLI, run:

```bash
cargo install --git http://github.com/steve-kaufman/simple-rust-cov
```

> Crates.io package in the works

## Usage

### CLI Options:

To get CLI options, run:

```bash
simple-rust-cov --help
```

At the time of writing, that outputs:

```
Usage: simple-rust-cov [OPTIONS] [PROJECT_DIR]

Arguments:
  [PROJECT_DIR]  Path to Cargo project. Defaults to current working directory

Options:
      --min-line-coverage <MIN_LINE_COVERAGE>
      --min-branch-coverage <MIN_BRANCH_COVERAGE>
  -h, --help                                       Print help
  -V, --version
```

Note that min-line-coverage and min-branch-coverage are expected in decimal, not as a percentage, i.e. 1.0, not 100%.

## Why Use This?

I pretty much wrote this for myself, but as far as I can tell, there isn't a good standard way of getting a simple test coverage check using LLVM. I wrote this based off of [this page in the rustc book](https://doc.rust-lang.org/rustc/instrument-coverage.html) and [this article](https://eugene-babichenko.github.io/blog/rust-code-coverage-without-3rd-party-utilities/), which explain how to do what this CLI does more manually.

I was also frustrated that even after following these steps, the llvm-cov utility doesn't have a built-in way to set a coverage threshold and return a nonzero status code if a threshold isn't met. You have to do your own wonky parsing of the output to figure out what the percentage is and then manually make it fail if it's below your threshold. Basically, I just want this to be a simple check in a CI pipeline that cries if I have less than 100% coverage.

## Future Features

### 1. Exclusion

Just threw this current version together in a day, and I have not looked into how to exclude certain source files or functions or anything like that from the coverage report. That's definitely a feature that *I* will personally want, so I'm likely to add it soon.

### 2. More Output Formats

llvm-cov natively supports a bunch of output formats, which this CLI doesn't currently expose. It's pretty much hard-coded to just use the default text table output (with color :)). Probably this will be implemented by simply allowing a CLI arg that gets forwarded to llvm-cov and controls what output format it produces.

