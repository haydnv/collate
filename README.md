# collate
Rust collation utilities

Example usage:
```rust
use collate::*;

let collator = Collator::default();
let collection = [
    [1, 2, 3],
    [2, 3, 4],
    [3, 4, 5],
];

assert_eq!(collator.bisect_left(&collection, &[1]), 0);
assert_eq!(collator.bisect_right(&collection, &[1]), 1);
```
