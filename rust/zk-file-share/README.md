# Zero Knowledge: file sharing

Smart Contract for storing uploaded files as secret-shared variables,
to act as a file-sharing service of secret files.

### Usage

Users can upload dynamically sized files (as raw bytes),
which are stored in the ZK state of the contract. 

Owners of files can delete them, or change the ownership to share
the file with another user, who can retrieve the file.

To upload a file, the owner must publicly specify the size of
the file in bytes.
