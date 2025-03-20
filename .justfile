sea:
    just sea-reset
    just sea-generate

sea-reset: 
    cargo run --bin database -- refresh

sea-generate:
    sea-orm-cli generate entity -o ./crates/database/src/models