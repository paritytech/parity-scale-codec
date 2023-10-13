# Parity SCALE Codec fuzzer

## Requirements:

Make sure you have the requirements installed:

https://github.com/rust-fuzz/honggfuzz-rs#dependencies

Install [honggfuzz-rs](https://github.com/rust-fuzz/honggfuzz-rs):
```
cargo install honggfuzz
```

Run the fuzzer:
```
cargo hfuzz run codec-fuzzer
```
