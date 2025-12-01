# rustqlite

SQLite rewrite for a Rust discipline on the university.

## Pre-requisites

Make sure you have at least the version 1.90 of the rust compiler and cargo installed. You can do this by going into [rust-lang.org](https://rust-lang.org/tools/install/)

## Usage

The program is capable of doing a variety of select queries with the where clause. There's also support other sqlite utilities like `.tables` and `.dbinfo`.

```bash
cargo run -- sample.db .tables
```

```bash
cargo run -- sample.db .dbinfo
```

```bash
cargo run -- sample.db "SELECT name,description FROM apples"
```

```bash
cargo run -- sample.db "SELECT COUNT(*) FROM apples"
```

```bash
cargo run -- sample.db "SELECT * FROM apples where id = 3"
```

```bash
cargo run -- sample.db "SELECT * FROM apples WHERE id > 3 AND id < 10"
```

> [!NOTE]
> The keywords are not case sensitive, so you can use lower case keywords too.

For playing with more databases, you can use the script `download_sample_databases.sh` to download more.
