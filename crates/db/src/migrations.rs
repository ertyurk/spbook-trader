// Migration utilities and helpers

pub const INITIAL_SCHEMA: &str = include_str!("../../../migrations/001_initial_schema.sql");

pub fn get_migrations() -> Vec<(&'static str, &'static str)> {
    vec![
        ("001", INITIAL_SCHEMA),
    ]
}