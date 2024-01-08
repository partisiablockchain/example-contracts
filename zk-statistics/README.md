# PBC Rust ZK Example: Statistics

The contract allows users to input secret data up until a deadline after which any user can
start a computation of statistics on the input data.
The user submitted data is an age group from { 0-19, 20-39, 40-59, 60- },
a gender from { male, female, other } and a favorite color from { red, blue, green, yellow }.

The statistics computed are summations of each choice of each variable.